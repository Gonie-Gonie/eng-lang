param(
    [string] $ExtensionRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [string] $OutputRoot = ""
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $ExtensionRoot)
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $RepoRoot "build\editor-tests\textmate_tokens"
}

& (Join-Path $PSScriptRoot "build-grammar.ps1") -ExtensionRoot $ExtensionRoot -Check

$GrammarSourcePath = Join-Path $ExtensionRoot "syntaxes\eng.tmLanguage.source.json"
$GrammarPath = Join-Path $ExtensionRoot "syntaxes\eng.tmLanguage.json"
$ExpectedPath = Join-Path $ExtensionRoot "test\expected\grammar_tokens.json"
$FixtureRoot = Join-Path $ExtensionRoot "test\grammar-fixtures"
$LspSourcePath = Join-Path $RepoRoot "crates\eng_lsp\src\lib.rs"
$CompilerUnitsPath = Join-Path $RepoRoot "crates\eng_compiler\src\units.rs"
$CompilerQuantitiesPath = Join-Path $RepoRoot "crates\eng_compiler\src\quantities.rs"
foreach ($RequiredPath in @($GrammarSourcePath, $GrammarPath, $ExpectedPath, $FixtureRoot, $LspSourcePath, $CompilerUnitsPath, $CompilerQuantitiesPath)) {
    if (-not (Test-Path -LiteralPath $RequiredPath)) {
        throw "missing grammar test input at $RequiredPath"
    }
}

$GrammarSourceRaw = Get-Content -LiteralPath $GrammarSourcePath -Raw -Encoding UTF8
$Grammar = Get-Content -LiteralPath $GrammarPath -Raw -Encoding UTF8 | ConvertFrom-Json
$ExpectedJson = Get-Content -LiteralPath $ExpectedPath -Raw -Encoding UTF8 | ConvertFrom-Json
$LspSource = Get-Content -LiteralPath $LspSourcePath -Raw -Encoding UTF8
$CompilerUnitsSource = Get-Content -LiteralPath $CompilerUnitsPath -Raw -Encoding UTF8
$CompilerQuantitiesSource = Get-Content -LiteralPath $CompilerQuantitiesPath -Raw -Encoding UTF8
$Expected = New-Object System.Collections.Generic.List[object]
if ($ExpectedJson -is [System.Array]) {
    foreach ($item in $ExpectedJson) {
        $Expected.Add($item) | Out-Null
    }
} else {
    $Expected.Add($ExpectedJson) | Out-Null
}
$PatternsByScope = @{}

function Add-ScopePattern {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][object] $Pattern,
        [object] $CaptureIndex = $null,
        [string] $CaptureKind = ""
    )

    if (-not $PatternsByScope.ContainsKey($Scope)) {
        $PatternsByScope[$Scope] = New-Object System.Collections.Generic.List[object]
    }
    $PatternsByScope[$Scope].Add([pscustomobject]@{
        pattern = $Pattern
        capture_index = $CaptureIndex
        capture_kind = $CaptureKind
    }) | Out-Null
}

function Add-CapturePatternNodes {
    param(
        [Parameter(Mandatory = $true)][object] $Node,
        [Parameter(Mandatory = $true)][string] $PropertyName,
        [Parameter(Mandatory = $true)][string] $CaptureKind
    )

    if ($Node.PSObject.Properties.Name -notcontains $PropertyName) {
        return
    }
    $captures = $Node.$PropertyName
    if ($null -eq $captures) {
        return
    }
    foreach ($capture in $captures.PSObject.Properties) {
        $captureNode = $capture.Value
        if ($null -ne $captureNode.name) {
            Add-ScopePattern -Scope ([string] $captureNode.name) -Pattern $Node -CaptureIndex ([int] $capture.Name) -CaptureKind $CaptureKind
        }
        if ($null -ne $captureNode.patterns) {
            foreach ($child in @($captureNode.patterns)) {
                Add-PatternNode $child
            }
        }
    }
}

function Add-PatternNode {
    param([object] $Node)

    if ($null -eq $Node) {
        return
    }
    if ($null -ne $Node.name) {
        Add-ScopePattern -Scope ([string] $Node.name) -Pattern $Node
    }
    Add-CapturePatternNodes -Node $Node -PropertyName "captures" -CaptureKind "match"
    Add-CapturePatternNodes -Node $Node -PropertyName "beginCaptures" -CaptureKind "begin"
    Add-CapturePatternNodes -Node $Node -PropertyName "endCaptures" -CaptureKind "end"
    if ($null -ne $Node.patterns) {
        foreach ($child in @($Node.patterns)) {
            Add-PatternNode $child
        }
    }
    if ($null -ne $Node.repository) {
        foreach ($property in $Node.repository.PSObject.Properties) {
            Add-PatternNode $property.Value
        }
    }
}

Add-PatternNode $Grammar

function Test-PatternMatchesText {
    param(
        [Parameter(Mandatory = $true)][object] $Pattern,
        [Parameter(Mandatory = $true)][string] $Text,
        [Parameter(Mandatory = $true)][string] $FixtureText,
        [bool] $FullMatch = $false
    )

    $patternNode = $Pattern.pattern
    $captureIndex = $Pattern.capture_index
    $captureKind = [string] $Pattern.capture_kind

    if ($null -ne $captureIndex) {
        $patternText = $null
        if ($captureKind -eq "begin" -and $null -ne $patternNode.begin) {
            $patternText = [string] $patternNode.begin
        } elseif ($captureKind -eq "end" -and $null -ne $patternNode.end) {
            $patternText = [string] $patternNode.end
        } elseif ($captureKind -eq "match" -and $null -ne $patternNode.match) {
            $patternText = [string] $patternNode.match
        }
        if ($null -eq $patternText) {
            return $false
        }
        foreach ($match in [regex]::Matches($FixtureText, $patternText, [System.Text.RegularExpressions.RegexOptions]::Multiline)) {
            if ($captureIndex -ge $match.Groups.Count) {
                continue
            }
            $capturedText = $match.Groups[$captureIndex].Value
            if ($FullMatch) {
                if ($capturedText -eq $Text) {
                    return $true
                }
            } elseif ($capturedText.Contains($Text)) {
                return $true
            }
        }
        return $false
    }

    if ($null -ne $patternNode.match) {
        $match = [regex]::Match($Text, [string] $patternNode.match)
        if (-not $match.Success) {
            return $false
        }
        if ($FullMatch) {
            return $match.Value -eq $Text
        }
        return $true
    }
    if ($null -ne $patternNode.begin -and $null -ne $patternNode.end) {
        return [regex]::IsMatch($Text, [string] $patternNode.begin) -and [regex]::IsMatch($Text, [string] $patternNode.end)
    }
    return $false
}

function Read-RustStringSliceConst {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][string] $Name
    )

    $pattern = "const\s+$([regex]::Escape($Name))\s*:\s*&\[\&str\]\s*=\s*&\[(?<body>.*?)\];"
    $match = [regex]::Match($Source, $pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
    if (-not $match.Success) {
        throw "missing Rust string slice constant $Name"
    }
    return @([regex]::Matches($match.Groups["body"].Value, '"([^"]+)"') | ForEach-Object { $_.Groups[1].Value })
}

function Read-RustTupleFirstStringsConst {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][string] $Name
    )

    $pattern = "const\s+$([regex]::Escape($Name))\s*:\s*&\[\(\&str,\s*\&str\)\]\s*=\s*&\[(?<body>.*?)\];"
    $match = [regex]::Match($Source, $pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
    if (-not $match.Success) {
        throw "missing Rust tuple string constant $Name"
    }
    return @([regex]::Matches($match.Groups["body"].Value, '\(\s*"([^"]+)"\s*,') | ForEach-Object { $_.Groups[1].Value })
}

function Read-RustStructFieldStringsConst {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][string] $Name,
        [Parameter(Mandatory = $true)][string] $FieldName
    )

    $pattern = "(?:pub\s+)?const\s+$([regex]::Escape($Name))\s*:\s*&\[[^\]]+\]\s*=\s*&\[(?<body>.*?)\];"
    $match = [regex]::Match($Source, $pattern, [System.Text.RegularExpressions.RegexOptions]::Singleline)
    if (-not $match.Success) {
        throw "missing Rust struct array constant $Name"
    }
    $fieldPattern = [regex]::Escape($FieldName) + '\s*:\s*"([^"]+)"'
    return @([regex]::Matches($match.Groups["body"].Value, $fieldPattern) | ForEach-Object { $_.Groups[1].Value })
}

function Assert-GrammarSourceContainsLabels {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][string[]] $Labels,
        [Parameter(Mandatory = $true)][string] $Description
    )

    foreach ($Label in $Labels) {
        $pattern = "(?<![A-Za-z0-9_])$([regex]::Escape($Label))(?![A-Za-z0-9_])"
        $regexEscapedLabel = [regex]::Escape($Label)
        $jsonEscapedRegexLabel = $regexEscapedLabel.Replace("\", "\\")
        $escapedPattern = "(?<![A-Za-z0-9_])$([regex]::Escape($jsonEscapedRegexLabel))(?![A-Za-z0-9_])"
        if ($Source -notmatch $pattern -and $Source -notmatch $escapedPattern) {
            throw "TextMate grammar source missing $Description label $Label"
        }
    }
}

function Assert-ScopeMatchesLabels {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string[]] $Labels,
        [Parameter(Mandatory = $true)][string] $Description,
        [string] $Suffix = ""
    )

    if (-not $PatternsByScope.ContainsKey($Scope)) {
        throw "grammar does not define scope $Scope"
    }

    foreach ($Label in $Labels) {
        $matched = $false
        $sample = "$Label$Suffix"
        foreach ($pattern in $PatternsByScope[$Scope]) {
            $patternNode = $pattern.pattern
            if ($null -eq $patternNode.match) {
                continue
            }
            $match = [regex]::Match($sample, [string] $patternNode.match)
            if ($match.Success -and $match.Value -eq $Label) {
                $matched = $true
                break
            }
        }
        if (-not $matched) {
            throw "TextMate scope $Scope does not match $Description label $Label"
        }
    }
}

$CompletionKeywords = Read-RustStringSliceConst -Source $LspSource -Name "COMPLETION_KEYWORDS"
$WorkflowBuiltins = Read-RustStringSliceConst -Source $LspSource -Name "WORKFLOW_BUILTIN_KEYWORDS"
$WorkflowOptions = Read-RustTupleFirstStringsConst -Source $LspSource -Name "WORKFLOW_OPTION_COMPLETIONS"
$PublicTypes = @(Read-RustTupleFirstStringsConst -Source $LspSource -Name "PUBLIC_TYPE_COMPLETIONS" | ForEach-Object {
    ($_ -replace "\[.*$", "")
} | Select-Object -Unique)
$CompilerUnitSymbols = @(Read-RustStructFieldStringsConst -Source $CompilerUnitsSource -Name "UNIT_INFOS" -FieldName "symbol" | Where-Object {
    $_ -cmatch '^[\x20-\x7E]+$'
} | Select-Object -Unique)
$CompilerQuantityKinds = @(Read-RustStructFieldStringsConst -Source $CompilerQuantitiesSource -Name "QUANTITY_COMPLETIONS" -FieldName "quantity_kind" | Select-Object -Unique)

Assert-GrammarSourceContainsLabels -Source $GrammarSourceRaw -Labels $CompletionKeywords -Description "LSP completion keyword"
Assert-GrammarSourceContainsLabels -Source $GrammarSourceRaw -Labels $WorkflowBuiltins -Description "LSP workflow builtin"
Assert-GrammarSourceContainsLabels -Source $GrammarSourceRaw -Labels $WorkflowOptions -Description "LSP workflow option"
Assert-GrammarSourceContainsLabels -Source $GrammarSourceRaw -Labels $PublicTypes -Description "LSP public type"
Assert-GrammarSourceContainsLabels -Source $GrammarSourceRaw -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-GrammarSourceContainsLabels -Source $GrammarSourceRaw -Labels $CompilerQuantityKinds -Description "compiler quantity"
Assert-ScopeMatchesLabels -Scope "support.function.builtin.englang" -Labels $WorkflowBuiltins -Description "LSP workflow builtin"
Assert-ScopeMatchesLabels -Scope "support.type.englang" -Labels $PublicTypes -Description "LSP public type"
Assert-ScopeMatchesLabels -Scope "support.type.englang" -Labels $CompilerQuantityKinds -Description "compiler quantity"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.englang" -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.format.englang" -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-ScopeMatchesLabels -Scope "variable.parameter.property.englang" -Labels $WorkflowOptions -Description "LSP workflow option" -Suffix " ="

$Results = New-Object System.Collections.Generic.List[object]
foreach ($case in $Expected) {
    $fixturePath = Join-Path $FixtureRoot $case.fixture
    if (-not (Test-Path -LiteralPath $fixturePath -PathType Leaf)) {
        throw "missing grammar fixture $($case.fixture)"
    }
    $fixtureText = Get-Content -LiteralPath $fixturePath -Raw -Encoding UTF8
    if (-not $fixtureText.Contains([string] $case.text)) {
        throw "fixture $($case.fixture) does not contain expected text '$($case.text)'"
    }
    if (-not $PatternsByScope.ContainsKey([string] $case.scope)) {
        throw "grammar does not define expected scope $($case.scope)"
    }
    $fullMatch = $false
    if ($case.PSObject.Properties.Name -contains "fullMatch") {
        $fullMatch = [bool] $case.fullMatch
    }
    $matched = $false
    foreach ($pattern in $PatternsByScope[[string] $case.scope]) {
        if (Test-PatternMatchesText -Pattern $pattern -Text ([string] $case.text) -FixtureText $fixtureText -FullMatch $fullMatch) {
            $matched = $true
            break
        }
    }
    if (-not $matched) {
        throw "scope $($case.scope) did not match '$($case.text)' from $($case.fixture)"
    }
    $Results.Add([ordered]@{
        fixture = $case.fixture
        scope = $case.scope
        text = $case.text
        full_match = $fullMatch
        status = "passed"
    }) | Out-Null
}

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
$ResultPath = Join-Path $OutputRoot "grammar_smoke.json"
$ResultJson = ($Results | ConvertTo-Json -Depth 8) + "`n"
[System.IO.File]::WriteAllText($ResultPath, $ResultJson, $Utf8NoBom)
Write-Host "VS Code grammar smoke passed. Checked $($Results.Count) token expectation(s)."
Write-Host "Grammar smoke output: $ResultPath"

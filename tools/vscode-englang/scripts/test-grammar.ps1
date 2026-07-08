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
$EditorMetadataPath = Join-Path $ExtensionRoot "generated\editor\englang-editor-metadata.json"
foreach ($RequiredPath in @($GrammarSourcePath, $GrammarPath, $ExpectedPath, $FixtureRoot, $EditorMetadataPath)) {
    if (-not (Test-Path -LiteralPath $RequiredPath)) {
        throw "missing grammar test input at $RequiredPath"
    }
}

$GrammarSourceRaw = Get-Content -LiteralPath $GrammarSourcePath -Raw -Encoding UTF8
$GrammarSource = $GrammarSourceRaw | ConvertFrom-Json
$GrammarGeneratedRaw = Get-Content -LiteralPath $GrammarPath -Raw -Encoding UTF8
$Grammar = Get-Content -LiteralPath $GrammarPath -Raw -Encoding UTF8 | ConvertFrom-Json
$ExpectedJson = Get-Content -LiteralPath $ExpectedPath -Raw -Encoding UTF8 | ConvertFrom-Json
$EditorMetadata = Get-Content -LiteralPath $EditorMetadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
$SyntaxCatalog = $EditorMetadata.syntax_catalog
if ($null -eq $SyntaxCatalog) {
    throw "generated editor metadata is missing syntax_catalog. Run .\dev.bat vscode-build-editor-metadata"
}
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
        if (-not [regex]::IsMatch($Text, [string] $patternNode.begin)) {
            return $false
        }
        if ([regex]::IsMatch($Text, [string] $patternNode.end)) {
            return $true
        }
        $textIndex = $FixtureText.IndexOf($Text)
        if ($textIndex -lt 0) {
            return $false
        }
        $remainingFixtureText = $FixtureText.Substring($textIndex + $Text.Length)
        return [regex]::IsMatch($remainingFixtureText, [string] $patternNode.end)
    }
    return $false
}

function Read-GrammarWorkflowOptionLabels {
    if (-not $PatternsByScope.ContainsKey("variable.parameter.property.englang")) {
        throw "grammar does not define workflow option property scope"
    }

    $labels = New-Object System.Collections.Generic.List[string]
    foreach ($pattern in $PatternsByScope["variable.parameter.property.englang"]) {
        $patternNode = $pattern.pattern
        if ($null -eq $patternNode.match) {
            continue
        }
        $matchText = [string] $patternNode.match
        if (-not $matchText.Contains("expected_sha256") -or -not $matchText.Contains("(?=\s*=)")) {
            continue
        }
        $start = $matchText.IndexOf('\b(')
        if ($start -lt 0) {
            continue
        }
        $end = $matchText.IndexOf(')\b(?=\s*=)', $start)
        if ($end -lt 0) {
            continue
        }
        $body = $matchText.Substring($start + 3, $end - ($start + 3))
        foreach ($label in @($body -split '\|')) {
            if (-not [string]::IsNullOrWhiteSpace($label)) {
                $labels.Add($label) | Out-Null
            }
        }
    }

    return @($labels | Sort-Object -Unique)
}

function Assert-WorkflowOptionsAreScopedToWithBlocks {
    $withOptions = $Grammar.repository.withOptions
    if ($null -eq $withOptions -or $null -eq $withOptions.patterns) {
        throw "grammar does not define a withOptions repository"
    }

    $withBlock = @($withOptions.patterns | Where-Object {
        $_.name -eq "meta.workflow.with-block.englang" -and $_.begin -eq "\b(with)\s*(\{)"
    })
    if ($withBlock.Count -ne 1) {
        throw "workflow option labels must be scoped under one meta.workflow.with-block.englang pattern"
    }

    $topLevelOptionMatchers = @($withOptions.patterns | Where-Object {
        $_.name -eq "variable.parameter.property.englang" -and $null -ne $_.match
    })
    if ($topLevelOptionMatchers.Count -gt 0) {
        throw "workflow option label matchers must not live at top-level withOptions scope"
    }
}

function Assert-UnitsPrecedeBuiltinsInIncludeGroups {
    param(
        [Parameter(Mandatory = $true)][object] $Node,
        [string] $Path = "grammar"
    )

    if ($null -eq $Node) {
        return
    }
    if ($null -ne $Node.patterns) {
        $patterns = @($Node.patterns)
        $includes = @($patterns | Where-Object { $null -ne $_.include } | ForEach-Object { [string]$_.include })
        $unitIndex = [array]::IndexOf($includes, "#units")
        $builtinIndex = [array]::IndexOf($includes, "#builtins")
        if ($unitIndex -ge 0 -and $builtinIndex -ge 0 -and $unitIndex -gt $builtinIndex) {
            throw "TextMate include order must place #units before #builtins at $Path"
        }
        for ($i = 0; $i -lt $patterns.Count; $i++) {
            Assert-UnitsPrecedeBuiltinsInIncludeGroups -Node $patterns[$i] -Path "$Path.patterns[$i]"
        }
    }
    if ($null -ne $Node.repository) {
        foreach ($property in $Node.repository.PSObject.Properties) {
            Assert-UnitsPrecedeBuiltinsInIncludeGroups -Node $property.Value -Path "$Path.repository.$($property.Name)"
        }
    }
}

function Assert-GeneratedGrammarContainsLabels {
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
            throw "TextMate generated grammar missing $Description label $Label"
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

function Assert-ScopeDoesNotMatchLabels {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string[]] $Labels,
        [Parameter(Mandatory = $true)][string] $Description,
        [string] $Suffix = ""
    )

    if (-not $PatternsByScope.ContainsKey($Scope)) {
        return
    }

    foreach ($Label in $Labels) {
        $fixtureText = "$Label$Suffix"
        foreach ($pattern in $PatternsByScope[$Scope]) {
            if (Test-PatternMatchesLabelInFixture -Pattern $pattern -Label $Label -FixtureText $fixtureText) {
                throw "TextMate scope $Scope must not match $Description label $Label"
            }
        }
    }
}

function Assert-ScopeDoesNotMatchText {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string] $Text,
        [Parameter(Mandatory = $true)][string] $Description
    )

    if (-not $PatternsByScope.ContainsKey($Scope)) {
        return
    }

    foreach ($pattern in $PatternsByScope[$Scope]) {
        if (Test-PatternMatchesText -Pattern $pattern -Text $Text -FixtureText $Text -FullMatch $true) {
            throw "TextMate scope $Scope must not match $Description text '$Text'"
        }
    }
}

function Test-PatternMatchesLabelInFixture {
    param(
        [Parameter(Mandatory = $true)][object] $Pattern,
        [Parameter(Mandatory = $true)][string] $Label,
        [Parameter(Mandatory = $true)][string] $FixtureText
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
            if ($captureIndex -lt $match.Groups.Count -and $match.Groups[$captureIndex].Value -eq $Label) {
                return $true
            }
        }
        return $false
    }

    if ($null -ne $patternNode.match) {
        foreach ($match in [regex]::Matches($FixtureText, [string] $patternNode.match, [System.Text.RegularExpressions.RegexOptions]::Multiline)) {
            if ($match.Value -eq $Label) {
                return $true
            }
        }
        $match = [regex]::Match($Label, [string] $patternNode.match)
        return $match.Success -and $match.Value -eq $Label
    }

    return $false
}

function Test-PatternMatchesLabelOnlyInFixture {
    param(
        [Parameter(Mandatory = $true)][object] $Pattern,
        [Parameter(Mandatory = $true)][string] $Label,
        [Parameter(Mandatory = $true)][string] $FixtureText
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
            if ($captureIndex -lt $match.Groups.Count -and $match.Groups[$captureIndex].Value -eq $Label) {
                return $true
            }
        }
        return $false
    }

    if ($null -ne $patternNode.match) {
        foreach ($match in [regex]::Matches($FixtureText, [string] $patternNode.match, [System.Text.RegularExpressions.RegexOptions]::Multiline)) {
            if ($match.Value -eq $Label) {
                return $true
            }
        }
    }
    return $false
}

function Assert-ScopeDoesNotMatchLabelInFixture {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string] $Label,
        [Parameter(Mandatory = $true)][string] $FixtureText,
        [Parameter(Mandatory = $true)][string] $Description
    )

    if (-not $PatternsByScope.ContainsKey($Scope)) {
        return
    }
    foreach ($pattern in $PatternsByScope[$Scope]) {
        if (Test-PatternMatchesLabelOnlyInFixture -Pattern $pattern -Label $Label -FixtureText $FixtureText) {
            throw "TextMate scope $Scope must not match $Description label $Label in '$FixtureText'"
        }
    }
}

function Assert-AnyScopeMatchesLabels {
    param(
        [Parameter(Mandatory = $true)][string[]] $Scopes,
        [Parameter(Mandatory = $true)][string[]] $Labels,
        [Parameter(Mandatory = $true)][string] $Description,
        [Parameter(Mandatory = $true)][string] $FixtureText
    )

    foreach ($Label in $Labels) {
        $matched = $false
        foreach ($Scope in $Scopes) {
            if (-not $PatternsByScope.ContainsKey($Scope)) {
                continue
            }
            foreach ($pattern in $PatternsByScope[$Scope]) {
                if (Test-PatternMatchesLabelInFixture -Pattern $pattern -Label $Label -FixtureText $FixtureText) {
                    $matched = $true
                    break
                }
            }
            if ($matched) {
                break
            }
        }
        if (-not $matched) {
            throw "TextMate grammar does not color $Description label $Label with an accepted fallback scope"
        }
    }
}

function Assert-ExpectedWorkflowScopesCoverGrammar {
    $GrammarWorkflowScopes = @($PatternsByScope.Keys | Where-Object {
        [string]$_ -like "meta.workflow.*"
    } | Sort-Object -Unique)
    $ExpectedWorkflowScopes = @($Expected | ForEach-Object {
        [string]$_.scope
    } | Where-Object {
        $_ -like "meta.workflow.*"
    } | Sort-Object -Unique)

    $MissingWorkflowScopes = @($GrammarWorkflowScopes | Where-Object {
        $ExpectedWorkflowScopes -notcontains $_
    })
    if ($MissingWorkflowScopes.Count -gt 0) {
        throw "grammar smoke expected tokens missing workflow scope coverage: $($MissingWorkflowScopes -join ', ')"
    }
}

function Test-ExpectedTokenTextCoversLabel {
    param([Parameter(Mandatory = $true)][string] $Label)

    $pattern = "(?<![A-Za-z0-9_])$([regex]::Escape($Label))(?![A-Za-z0-9_])"
    foreach ($case in $Expected) {
        if ([string]$case.text -match $pattern) {
            return $true
        }
    }
    return $false
}

function Assert-ExpectedTokenTextsCoverLabels {
    param(
        [Parameter(Mandatory = $true)][string[]] $Labels,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $missing = @($Labels | Where-Object { -not (Test-ExpectedTokenTextCoversLabel -Label $_) } | Sort-Object -Unique)
    if ($missing.Count -gt 0) {
        throw "grammar smoke expected tokens missing $Description text coverage: $($missing -join ', ')"
    }
}

function Assert-WorkflowPatternIncludes {
    param(
        [Parameter(Mandatory = $true)][string] $Name,
        [Parameter(Mandatory = $true)][string] $Include,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $patterns = @($Grammar.repository.workflowPhrases.patterns | Where-Object {
        [string]$_.name -eq $Name
    })
    if ($patterns.Count -ne 1) {
        throw "TextMate grammar must define exactly one $Description workflow pattern"
    }
    $includes = @($patterns[0].patterns | Where-Object {
        $null -ne $_.include
    } | ForEach-Object {
        [string]$_.include
    })
    if ($includes -notcontains $Include) {
        throw "TextMate grammar $Description workflow pattern must include $Include"
    }
}

function Assert-FunctionCallFallbacks {
    $patterns = @($Grammar.repository.functionCalls.patterns | Where-Object {
        [string]$_.name -eq "meta.function-call.englang"
    })
    if ($patterns.Count -ne 1) {
        throw "TextMate grammar must define exactly one generic function-call pattern"
    }

    $includes = @($patterns[0].patterns | Where-Object {
        $null -ne $_.include
    } | ForEach-Object {
        [string]$_.include
    })
    foreach ($include in @("#operators", "#units", "#builtins", "#numbers", "#punctuation")) {
        if ($includes -notcontains $include) {
            throw "TextMate generic function-call pattern must include $include"
        }
    }

    $scopes = @($patterns[0].patterns | Where-Object {
        $null -ne $_.name
    } | ForEach-Object {
        [string]$_.name
    })
    foreach ($scope in @("variable.parameter.property.englang", "variable.other.property.englang", "variable.other.local.englang")) {
        if ($scopes -notcontains $scope) {
            throw "TextMate generic function-call pattern must include $scope fallback"
        }
    }
}

$CompletionKeywords = @($SyntaxCatalog.keywords | ForEach-Object { [string]$_ })
$WorkflowBuiltins = @($SyntaxCatalog.workflow_builtins | ForEach-Object { [string]$_ })
$HyphenatedWorkflowBuiltins = @($SyntaxCatalog.hyphenated_workflow_builtins | ForEach-Object { [string]$_ })
$WorkflowOptions = @($SyntaxCatalog.workflow_options | ForEach-Object { [string]$_.label })
$GrammarOnlyWorkflowBuiltinAliases = @(
    "regression_table",
    "train_regression"
)
$GrammarOnlyWorkflowOptionAliases = @(
    "fixture"
)
$GrammarWorkflowOptions = Read-GrammarWorkflowOptionLabels
$PublicTypeLabels = @($SyntaxCatalog.public_types | ForEach-Object { [string]$_.label })
$PublicGenericTypes = @($PublicTypeLabels | Where-Object {
    $_ -match "\[[^\]]+\]"
})
$PublicTypes = @($PublicTypeLabels | ForEach-Object {
    ($_ -replace "\[.*$", "")
} | Select-Object -Unique)
$CompilerUnitSymbols = @($SyntaxCatalog.units | ForEach-Object { [string]$_.label } | Where-Object {
    $_ -cmatch '^[\x20-\x7E]+$'
} | Select-Object -Unique)
$CompilerQuantityKinds = @($SyntaxCatalog.quantities | ForEach-Object { [string]$_.label } | Select-Object -Unique)

Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $CompletionKeywords -Description "LSP completion keyword"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $WorkflowBuiltins -Description "LSP workflow builtin"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $HyphenatedWorkflowBuiltins -Description "LSP hyphenated workflow builtin"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $GrammarOnlyWorkflowBuiltinAliases -Description "grammar-only workflow builtin alias"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $WorkflowOptions -Description "LSP workflow option"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $GrammarOnlyWorkflowOptionAliases -Description "grammar-only workflow option alias"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $PublicTypes -Description "LSP public type"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $CompilerQuantityKinds -Description "compiler quantity"
Assert-UnitsPrecedeBuiltinsInIncludeGroups -Node $GrammarSource.grammar
Assert-WorkflowOptionsAreScopedToWithBlocks
$AllowedGrammarWorkflowOptions = @($WorkflowOptions + $GrammarOnlyWorkflowOptionAliases)
$GrammarOptionsMissingFromLsp = @($GrammarWorkflowOptions | Where-Object { $AllowedGrammarWorkflowOptions -notcontains $_ } | Sort-Object -Unique)
if ($GrammarOptionsMissingFromLsp.Count -gt 0) {
    throw "TextMate workflow option labels are missing from LSP workflow options: $($GrammarOptionsMissingFromLsp -join ', ')"
}
$LspOptionsMissingFromGrammar = @($WorkflowOptions | Where-Object { $GrammarWorkflowOptions -notcontains $_ } | Sort-Object -Unique)
if ($LspOptionsMissingFromGrammar.Count -gt 0) {
    throw "LSP workflow options are missing from TextMate workflow option labels: $($LspOptionsMissingFromGrammar -join ', ')"
}
$CompletionKeywordFixture = @'
use eng.path
import eng.table
from eng.std import symbol as alias

schema GuardSchema {
    index time: DateTime [iso8601]
    value: HeatRate [kW]
}

class GuardClass {
    method label() -> String = "guard"
}

fn guard_fn() -> HeatRate {
    return 1 kW
}

system GuardSystem {
    state T: AbsoluteTemperature [K]
    input load: HeatRate [kW]
    parameter C: HeatCapacity [J/K]
    output Q: HeatRate [kW]
    equation Q eq load
}

domain GuardDomain package "eng.std.domains.guard" version "0.1.0" {
    across T: AbsoluteTemperature [K]
    through q: HeatRate [W]
    conservation sum(q) = 0 W
}

component GuardComponent {
    port heat: GuardDomain
}

connect GuardComponent.heat -> GuardComponent.heat
constraints {
    validate Q within 1 kW
}

args {
    input: CsvFile = file("data/input.csv")
}

missing {
    method = interpolate
}

test "guard" {
    assert true
    golden file("expected.txt") matches file("actual.txt") within 1
}

with {
    mode = append
    policy = keep
    missing = interpolate
}

records_table = promote json records payload.records as GuardSchema
csv_table = promote csv args.input as GuardSchema
raw_json = read json file("data/input.json")
raw_toml = read toml file("data/input.toml")
raw_text = read text file("data/input.txt")
write text file("outputs/out.txt"), "ok"
export summary to csv file("outputs/summary.csv")
copy file("a.txt") to file("b.txt")
mkdir dir("out")
move file("b.txt") to file("c.txt")
delete file("c.txt")
render template file("template.txt")
run command "tool"
http get url("https://example.org")
http post url("https://example.org")
http put url("https://example.org")
http patch url("https://example.org")
http head url("https://example.org")
http request url("https://example.org")
http fetch url("https://example.org")
download url("https://example.org/file.csv") to file("outputs/file.csv")
db = open sqlite file("outputs/run.sqlite")
materialized = materialize cases csv_table
case_results = apply run_case over materialized
collected = collect results case_results
samples = sample lhs
uniform_samples = sample uniform
split = train_test_split(Q, target=Q, features=[Q], test=0.25, seed=7)
reg = regression(split)
trained_table = train regression csv_table
reg_table = regression_table(csv_table, target=Q, features=[value], test=0.25, seed=7)
nn = mlp(split)
metrics = evaluate(reg, split=split)
card = model_card(reg)
leakage = leakage_lint(split)
predictions = predict reg using csv_table
selected = select_first_row(csv_table)
filtered = filter csv_table
projected = select csv_table
joined = join csv_table with filtered
on {
    csv_table.time == filtered.time
}
derived = derive csv_table column copy = value
sorted = sort csv_table by value
one = require_one filtered
covered = check coverage one.time
aligned = align Q to Time
resampled = resample Q by 1 h
filled = fill_missing Q
meas = measured(1 kW, std=0.1 kW)
span = interval(0 kW, 1 kW)
ens = ensemble([1 kW, 2 kW])
prop = propagate Q
prob = probability Q > 0 kW
avg = mean(Q)
tw = time_weighted_mean(Q)
lo = min(Q)
hi = max(Q)
mid = median(Q)
sigma = std(Q)
q90 = p90(Q)
q95 = p95(Q)
err = rmse(Q, Q)
above = duration_above(Q, 1 kW)
energy = integrate Q over Time
rate = der(Q)
lag = delay(Q, 1 h)
total = sum(Q)

report {
    show Q
    plot Q over Time line
    plot Q over Time bar
    plot Q over Time histogram
    summarize Q
    summary Q
    distribution Q
}

simulate GuardSystem
solve GuardSystem
if true else false
model = reg
none null and or not between over by using in into is where of vs
append insert upsert replace commit rollback keep empty interpolate monotonic safe normal repro
'@
$KeywordFallbackScopes = @(
    "keyword.control.import.englang",
    "keyword.control.deprecated.englang",
    "keyword.control.report.englang",
    "keyword.control.validation.englang",
    "keyword.control.side-effect.englang",
    "keyword.control.external-boundary.englang",
    "keyword.control.solver.englang",
    "keyword.control.workflow.englang",
    "keyword.operator.word.englang",
    "constant.language.englang",
    "storage.type.declaration.englang",
    "storage.type.function.englang",
    "storage.modifier.englang",
    "storage.type.test.englang",
    "storage.type.block.englang",
    "storage.modifier.schema.englang",
    "support.function.builtin.englang",
    "variable.parameter.property.englang"
)
Assert-AnyScopeMatchesLabels -Scopes $KeywordFallbackScopes -Labels $CompletionKeywords -Description "LSP completion keyword" -FixtureText $CompletionKeywordFixture
Assert-ScopeMatchesLabels -Scope "support.function.builtin.englang" -Labels $WorkflowBuiltins -Description "LSP workflow builtin"
Assert-ScopeMatchesLabels -Scope "support.function.builtin.englang" -Labels $HyphenatedWorkflowBuiltins -Description "LSP hyphenated workflow builtin"
Assert-ScopeDoesNotMatchLabels -Scope "entity.name.function.call.englang" -Labels ($WorkflowBuiltins + $HyphenatedWorkflowBuiltins) -Description "LSP workflow builtin call" -Suffix "("
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.read-structured.englang" -Text 'read csv file("data/input.csv")' -Description "unsupported raw CSV read"
Assert-ScopeMatchesLabels -Scope "support.type.englang" -Labels $PublicTypes -Description "LSP public type"
Assert-ScopeMatchesLabels -Scope "meta.type.generic.englang" -Labels $PublicGenericTypes -Description "LSP public generic type"
Assert-ScopeMatchesLabels -Scope "support.type.englang" -Labels $CompilerQuantityKinds -Description "compiler quantity"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.englang" -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.format.englang" -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-ScopeDoesNotMatchLabelInFixture -Scope "constant.other.unit.englang" -Label "min" -FixtureText "min(Q_series)" -Description "function-call"
Assert-ScopeDoesNotMatchLabelInFixture -Scope "constant.other.unit.englang" -Label "min" -FixtureText "min (Q_series)" -Description "function-call"
Assert-ScopeMatchesLabels -Scope "variable.parameter.property.englang" -Labels $WorkflowOptions -Description "LSP workflow option" -Suffix " ="
Assert-ExpectedTokenTextsCoverLabels -Labels $CompletionKeywords -Description "generated keyword"
Assert-ExpectedTokenTextsCoverLabels -Labels $HyphenatedWorkflowBuiltins -Description "hyphenated workflow builtin"
Assert-ExpectedWorkflowScopesCoverGrammar
Assert-WorkflowPatternIncludes -Name "meta.workflow.render-template.englang" -Include "#operators" -Description "render template"
Assert-WorkflowPatternIncludes -Name "meta.workflow.download-to.englang" -Include "#operators" -Description "download"
Assert-FunctionCallFallbacks

function Resolve-GrammarFixturePath {
    param([Parameter(Mandatory = $true)][string] $Fixture)

    $fixturePath = Join-Path $FixtureRoot $Fixture
    if (Test-Path -LiteralPath $fixturePath -PathType Leaf) {
        return $fixturePath
    }

    $repoFixturePath = Join-Path $RepoRoot $Fixture
    if (Test-Path -LiteralPath $repoFixturePath -PathType Leaf) {
        return $repoFixturePath
    }

    throw "missing grammar fixture $Fixture"
}

$Results = New-Object System.Collections.Generic.List[object]
foreach ($case in $Expected) {
    $fixturePath = Resolve-GrammarFixturePath -Fixture ([string] $case.fixture)
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

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
$PackagePath = Join-Path $ExtensionRoot "package.json"
foreach ($RequiredPath in @($GrammarSourcePath, $GrammarPath, $ExpectedPath, $FixtureRoot, $EditorMetadataPath, $PackagePath)) {
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
$PackageJson = Get-Content -LiteralPath $PackagePath -Raw -Encoding UTF8 | ConvertFrom-Json
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
        $patternText = [string] $patternNode.match
        $match = [regex]::Match($Text, $patternText)
        if ($FullMatch) {
            if ($match.Success -and $match.Value -eq $Text) {
                return $true
            }
            foreach ($fixtureMatch in [regex]::Matches($FixtureText, $patternText, [System.Text.RegularExpressions.RegexOptions]::Multiline)) {
                if ($fixtureMatch.Value -eq $Text) {
                    return $true
                }
            }
            return $false
        }
        return $match.Success
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
                $labels.Add([regex]::Unescape($label)) | Out-Null
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

function Assert-ExpectedLeafScopesCoverGrammar {
    $RootGrammarName = [string]$Grammar.name
    # Generic builtin coloring is an error-tolerant fallback for incomplete
    # editor states. Public helper fixtures must use a role-specific scope.
    $SyntheticFallbackScopes = @("support.function.builtin.englang")
    $GrammarLeafScopes = @($PatternsByScope.Keys | Where-Object {
        $scope = [string]$_
        -not [string]::IsNullOrWhiteSpace($scope) -and
            $scope -ne $RootGrammarName -and
            $scope -notlike "meta.*" -and
            $SyntheticFallbackScopes -notcontains $scope
    } | Sort-Object -Unique)
    $ExpectedLeafScopes = @($Expected | ForEach-Object {
        [string]$_.scope
    } | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_) -and $_ -notlike "meta.*"
    } | Sort-Object -Unique)

    $MissingLeafScopes = @($GrammarLeafScopes | Where-Object {
        $ExpectedLeafScopes -notcontains $_
    })
    if ($MissingLeafScopes.Count -gt 0) {
        throw "grammar smoke expected tokens missing leaf scope coverage: $($MissingLeafScopes -join ', ')"
    }
}

function Assert-ActualWorkflowPublicMemberExpectedTokens {
    $RequiredActualWorkflowTokens = @(
        @{ Fixture = "examples/workflows/01_weather_api_to_standard_file/main.eng"; Scope = "meta.path.public-member.englang"; Text = "api_response.url_with_query" },
        @{ Fixture = "examples/workflows/01_weather_api_to_standard_file/main.eng"; Scope = "variable.other.public-member.englang"; Text = "url_with_query" },
        @{ Fixture = "examples/workflows/01_weather_api_to_standard_file/main.eng"; Scope = "variable.other.public-member.englang"; Text = "response_source" },
        @{ Fixture = "examples/workflows/01_weather_api_to_standard_file/main.eng"; Scope = "meta.path.public-member.englang"; Text = "coverage.actual_count" },
        @{ Fixture = "examples/workflows/01_weather_api_to_standard_file/main.eng"; Scope = "variable.other.public-member.englang"; Text = "actual_count" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "meta.path.public-member.englang"; Text = "training_designs.row_preview" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "variable.other.public-member.englang"; Text = "row_preview" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "meta.path.public-member.englang"; Text = "case_inputs.rendered_count" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "variable.other.public-member.englang"; Text = "rendered_count" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "meta.path.public-member.englang"; Text = "db.tables_written" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "variable.other.public-member.englang"; Text = "tables_written" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "meta.path.public-member.englang"; Text = "db.row_count" },
        @{ Fixture = "examples/workflows/02_native_surrogate_case_workflow/main.eng"; Scope = "variable.other.public-member.englang"; Text = "row_count" },
        @{ Fixture = "examples/workflows/03_uncertain_sensor_report/main.eng"; Scope = "meta.path.public-member.englang"; Text = "sensor.rows" },
        @{ Fixture = "examples/workflows/03_uncertain_sensor_report/main.eng"; Scope = "variable.other.public-member.englang"; Text = "rows" },
        @{ Fixture = "examples/workflows/03_uncertain_sensor_report/main.eng"; Scope = "meta.path.public-member.englang"; Text = "coverage.missing_count" },
        @{ Fixture = "examples/workflows/03_uncertain_sensor_report/main.eng"; Scope = "variable.other.public-member.englang"; Text = "missing_count" }
    )

    foreach ($RequiredToken in $RequiredActualWorkflowTokens) {
        $Matches = @($Expected | Where-Object {
            [string]$_.fixture -eq [string]$RequiredToken.Fixture -and
            [string]$_.scope -eq [string]$RequiredToken.Scope -and
            [string]$_.text -eq [string]$RequiredToken.Text
        })
        if ($Matches.Count -lt 1) {
            throw "grammar smoke expected tokens must cover actual workflow public member $($RequiredToken.Text) with $($RequiredToken.Scope) in $($RequiredToken.Fixture)"
        }
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

function Test-ExpectedScopedTokenCoversLabel {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string] $Label
    )

    foreach ($case in $Expected) {
        if ([string]$case.scope -eq $Scope -and [string]$case.text -eq $Label) {
            return $true
        }
    }
    return $false
}

function Assert-ExpectedScopedTokenTextsCoverLabels {
    param(
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string[]] $Labels,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $missing = @($Labels | Where-Object { -not (Test-ExpectedScopedTokenCoversLabel -Scope $Scope -Label $_) } | Sort-Object -Unique)
    if ($missing.Count -gt 0) {
        throw "grammar smoke expected tokens missing $Description scoped coverage for ${Scope}: $($missing -join ', ')"
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

function Assert-WorkflowBeginCaptureScope {
    param(
        [Parameter(Mandatory = $true)][string] $Name,
        [Parameter(Mandatory = $true)][string] $Begin,
        [Parameter(Mandatory = $true)][string] $CaptureIndex,
        [Parameter(Mandatory = $true)][string] $Scope,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $patterns = @($Grammar.repository.workflowPhrases.patterns | Where-Object {
        [string]$_.name -eq $Name -and [string]$_.begin -eq $Begin
    })
    if ($patterns.Count -ne 1) {
        throw "TextMate grammar must define exactly one $Description workflow begin pattern"
    }

    $captures = $patterns[0].beginCaptures
    if ($null -eq $captures -or $captures.PSObject.Properties.Name -notcontains $CaptureIndex) {
        throw "TextMate grammar $Description workflow pattern must define begin capture $CaptureIndex"
    }

    $actualScope = [string]$captures.$CaptureIndex.name
    if ($actualScope -ne $Scope) {
        throw "TextMate grammar $Description workflow begin capture $CaptureIndex must use $Scope, got $actualScope"
    }
}

function Assert-BeginEndWorkflowPhrasesAreMemberAware {
    $offenders = New-Object System.Collections.Generic.List[string]
    foreach ($pattern in @($GrammarSource.grammar.repository.workflowPhrases.patterns)) {
        $name = [string]$pattern.name
        if ($name -notlike "meta.workflow.*") {
            continue
        }
        if ($null -eq $pattern.begin -or $null -eq $pattern.end) {
            continue
        }
        $includes = @($pattern.patterns | Where-Object {
            $null -ne $_.include
        } | ForEach-Object {
            [string]$_.include
        })
        if ($includes -notcontains "#members") {
            $offenders.Add($name) | Out-Null
        }
    }

    $uniqueOffenders = @($offenders | Sort-Object -Unique)
    if ($uniqueOffenders.Count -gt 0) {
        throw "TextMate begin/end workflow phrase patterns must include #members for member-aware first-paint highlighting: $($uniqueOffenders -join ', ')"
    }
}
function Assert-WorkflowPropertyFallbacksAreMemberAware {
    $allowedWithoutMembers = @(
        "meta.workflow.status-condition.englang",
        "meta.workflow.status-option.englang"
    )
    $offenders = New-Object System.Collections.Generic.List[string]

    function Visit-WorkflowPatternForMemberFallbacks {
        param([object] $Node)

        if ($null -eq $Node) {
            return
        }
        if ($Node -is [System.Array]) {
            foreach ($item in $Node) {
                Visit-WorkflowPatternForMemberFallbacks -Node $item
            }
            return
        }
        if ($Node -isnot [pscustomobject]) {
            return
        }

        $name = [string]$Node.name
        if ($name -like "meta.workflow.*") {
            $serialized = $Node | ConvertTo-Json -Depth 30 -Compress
            $usesPropertyFallback = $serialized.Contains("variable.parameter.property.englang") -or $serialized.Contains("variable.other.property.englang")
            $hasMembers = $serialized.Contains('"include":"#members"')
            if ($usesPropertyFallback -and -not $hasMembers -and $allowedWithoutMembers -notcontains $name) {
                $offenders.Add($name) | Out-Null
            }
        }

        foreach ($property in $Node.PSObject.Properties) {
            Visit-WorkflowPatternForMemberFallbacks -Node $property.Value
        }
    }

    Visit-WorkflowPatternForMemberFallbacks -Node $GrammarSource.grammar.repository.workflowPhrases
    $uniqueOffenders = @($offenders | Sort-Object -Unique)
    if ($uniqueOffenders.Count -gt 0) {
        throw "TextMate workflow property fallback patterns must include #members before broad dotted fallbacks: $($uniqueOffenders -join ', ')"
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

function Get-PatternOrderLabel {
    param([object] $Pattern)

    if ($null -ne $Pattern.include) {
        return "include:$($Pattern.include)"
    }
    if ($null -eq $Pattern.name) {
        return ""
    }

    $name = [string]$Pattern.name
    $match = if ($null -ne $Pattern.match) { [string]$Pattern.match } else { "" }
    if ($name -eq "variable.parameter.property.englang" -and $match.Contains("args\.")) {
        return "scope:variable.parameter.property.path"
    }
    if ($name -eq "variable.other.property.englang" -and $match.Contains("(?:\.")) {
        return "scope:variable.other.property.path"
    }
    return "scope:$name"
}

function Get-RequiredPatternByName {
    param(
        [Parameter(Mandatory = $true)][object[]] $Patterns,
        [Parameter(Mandatory = $true)][string] $Name,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $matches = @($Patterns | Where-Object { [string]$_.name -eq $Name })
    if ($matches.Count -ne 1) {
        throw "TextMate grammar must define exactly one $Description pattern"
    }
    return $matches[0]
}

function Assert-PatternOrderBefore {
    param(
        [Parameter(Mandatory = $true)][object[]] $Patterns,
        [Parameter(Mandatory = $true)][string] $BeforeLabel,
        [Parameter(Mandatory = $true)][string[]] $AfterLabels,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $labels = @($Patterns | ForEach-Object { Get-PatternOrderLabel $_ } | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $beforeIndex = [array]::IndexOf($labels, $BeforeLabel)
    if ($beforeIndex -lt 0) {
        throw "TextMate grammar $Description must include $BeforeLabel"
    }

    foreach ($afterLabel in $AfterLabels) {
        $afterIndex = [array]::IndexOf($labels, $afterLabel)
        if ($afterIndex -ge 0 -and $beforeIndex -gt $afterIndex) {
            throw "TextMate grammar $Description must place $BeforeLabel before $afterLabel"
        }
    }
}

function Assert-MemberPathFallbackOrder {
    $dottedFallbacks = @(
        "scope:variable.parameter.property.path",
        "scope:variable.other.property.path"
    )

    $stringPattern = Get-RequiredPatternByName -Patterns @($Grammar.repository.strings.patterns) -Name "string.quoted.double.englang" -Description "double-quoted string"
    $interpolationPattern = Get-RequiredPatternByName -Patterns @($stringPattern.patterns) -Name "meta.interpolation.englang" -Description "string interpolation"
    Assert-PatternOrderBefore -Patterns @($interpolationPattern.patterns) -BeforeLabel "include:#members" -AfterLabels $dottedFallbacks -Description "string interpolation member paths"

    $validationPattern = Get-RequiredPatternByName -Patterns @($Grammar.repository.workflowPhrases.patterns) -Name "meta.workflow.validation.englang" -Description "validation workflow"
    Assert-PatternOrderBefore -Patterns @($validationPattern.patterns) -BeforeLabel "include:#members" -AfterLabels $dottedFallbacks -Description "validation member paths"

    $functionPattern = Get-RequiredPatternByName -Patterns @($Grammar.repository.functionCalls.patterns) -Name "meta.function-call.englang" -Description "generic function-call"
    Assert-PatternOrderBefore -Patterns @($functionPattern.patterns) -BeforeLabel "include:#members" -AfterLabels $dottedFallbacks -Description "generic function-call member paths"

    $withPattern = Get-RequiredPatternByName -Patterns @($Grammar.repository.withOptions.patterns) -Name "meta.workflow.with-block.englang" -Description "with-block"
    Assert-PatternOrderBefore -Patterns @($withPattern.patterns) -BeforeLabel "include:#members" -AfterLabels $dottedFallbacks -Description "with-block member paths"
}

function Assert-WithBlockExpressionFallbacks {
    $withPattern = Get-RequiredPatternByName -Patterns @($Grammar.repository.withOptions.patterns) -Name "meta.workflow.with-block.englang" -Description "with-block"
    $optionMapPattern = Get-RequiredPatternByName -Patterns @($withPattern.patterns) -Name "meta.workflow.option-map.englang" -Description "with-block option map"

    foreach ($include in @("#operators", "#constants", "#types", "#numbers", "#punctuation")) {
        $withIncludes = @($withPattern.patterns | Where-Object { $null -ne $_.include } | ForEach-Object { [string]$_.include })
        if ($withIncludes -notcontains $include) {
            throw "TextMate with-block expression fallback must include $include"
        }
        $mapIncludes = @($optionMapPattern.patterns | Where-Object { $null -ne $_.include } | ForEach-Object { [string]$_.include })
        if ($mapIncludes -notcontains $include) {
            throw "TextMate with-block option-map expression fallback must include $include"
        }
    }

    Assert-PatternOrderBefore -Patterns @($withPattern.patterns) -BeforeLabel "include:#constants" -AfterLabels @(
        "scope:variable.other.local.englang"
    ) -Description "with-block constants before local fallback"
    Assert-PatternOrderBefore -Patterns @($withPattern.patterns) -BeforeLabel "include:#operators" -AfterLabels @(
        "scope:variable.other.local.englang"
    ) -Description "with-block operators before local fallback"
}
function Assert-WorkflowStatusOptionPattern {
    $withPattern = Get-RequiredPatternByName -Patterns @($GrammarSource.grammar.repository.withOptions.patterns) -Name "meta.workflow.with-block.englang" -Description "source with-block"
    $statusPattern = Get-RequiredPatternByName -Patterns @($withPattern.patterns) -Name "meta.workflow.status-option.englang" -Description "workflow status option"
    if ([string]$statusPattern.match -notmatch "\\b\(status\)\\b\\s\*\(=\)\\s\*\(\{\{WORKFLOW_STATUS_LITERALS\}\}\)\\b") {
        throw "workflow status option pattern must use generated WORKFLOW_STATUS_LITERALS"
    }
    foreach ($capture in @(
        @{ Index = "1"; Scope = "variable.parameter.property.englang" },
        @{ Index = "2"; Scope = "keyword.operator.englang" },
        @{ Index = "3"; Scope = "constant.language.englang" }
    )) {
        $captureNode = $statusPattern.captures.PSObject.Properties[$capture.Index].Value
        if ($null -eq $captureNode -or [string]$captureNode.name -ne $capture.Scope) {
            throw "workflow status option capture $($capture.Index) must be $($capture.Scope)"
        }
    }
    Assert-PatternOrderBefore -Patterns @($withPattern.patterns) -BeforeLabel "scope:meta.workflow.status-option.englang" -AfterLabels @(
        "scope:variable.parameter.property.englang",
        "scope:variable.other.property.englang",
        "scope:variable.other.local.englang",
        "include:#constants"
    ) -Description "workflow status option values"
}

function Read-ThemeTokenScopes {
    param([Parameter(Mandatory = $true)][string] $ThemePath)

    if (-not (Test-Path -LiteralPath $ThemePath -PathType Leaf)) {
        throw "missing VS Code theme at $ThemePath"
    }
    $theme = Get-Content -LiteralPath $ThemePath -Raw -Encoding UTF8 | ConvertFrom-Json
    $scopes = New-Object System.Collections.Generic.List[string]
    foreach ($rule in @($theme.tokenColors)) {
        foreach ($scope in @($rule.scope)) {
            $scopeText = [string]$scope
            if (-not [string]::IsNullOrWhiteSpace($scopeText)) {
                $scopes.Add($scopeText) | Out-Null
            }
        }
    }
    return @($scopes)
}

function Test-ThemeScopeCoversScope {
    param(
        [Parameter(Mandatory = $true)][string[]] $ThemeScopes,
        [Parameter(Mandatory = $true)][string] $Scope
    )

    foreach ($themeScope in $ThemeScopes) {
        if ($Scope -eq $themeScope -or $Scope.StartsWith("$themeScope.")) {
            return $true
        }
    }
    return $false
}

function Assert-BundledThemeLeafScopeCoverage {
    $leafScopes = @($Expected | ForEach-Object { [string]$_.scope } | Where-Object {
        -not [string]::IsNullOrWhiteSpace($_) -and $_ -notlike "meta.*"
    } | Sort-Object -Unique)
    foreach ($themeName in @("englang-dark-color-theme.json", "englang-light-color-theme.json")) {
        $themePath = Join-Path (Join-Path $ExtensionRoot "themes") $themeName
        $themeScopes = Read-ThemeTokenScopes -ThemePath $themePath
        $missing = @($leafScopes | Where-Object { -not (Test-ThemeScopeCoversScope -ThemeScopes $themeScopes -Scope $_) })
        if ($missing.Count -gt 0) {
            throw "$themeName tokenColors are missing EngLang leaf scope coverage: $($missing -join ', ')"
        }
    }
}
function Assert-SemanticTokenScopeIncludes {
    param(
        [Parameter(Mandatory = $true)][object] $Package,
        [Parameter(Mandatory = $true)][string] $Selector,
        [Parameter(Mandatory = $true)][string[]] $Scopes,
        [Parameter(Mandatory = $true)][string] $Description
    )

    $entries = @($Package.contributes.semanticTokenScopes | Where-Object { [string]$_.language -eq "englang" })
    if ($entries.Count -ne 1) {
        throw "package.json must define exactly one englang semanticTokenScopes contribution"
    }
    $selectorProperty = $entries[0].scopes.PSObject.Properties[$Selector]
    if ($null -eq $selectorProperty) {
        throw "package.json semanticTokenScopes is missing $Description selector $Selector"
    }
    $actualScopes = @($selectorProperty.Value | ForEach-Object { [string]$_ })
    $missingScopes = @($Scopes | Where-Object { $actualScopes -notcontains $_ })
    if ($missingScopes.Count -gt 0) {
        throw "package.json semantic selector $Selector is missing $Description fallback scopes: $($missingScopes -join ', ')"
    }
}
$CompletionKeywords = @($SyntaxCatalog.keywords | ForEach-Object { [string]$_ })
$WorkflowBuiltins = @($SyntaxCatalog.workflow_builtins | ForEach-Object { [string]$_ })
$HyphenatedWorkflowBuiltins = @($SyntaxCatalog.hyphenated_workflow_builtins | ForEach-Object { [string]$_ })
$LegacyWorkflowBuiltinAliases = @($SyntaxCatalog.legacy_workflow_builtin_aliases | ForEach-Object { [string]$_ })
$WorkflowBuiltinGroups = $SyntaxCatalog.workflow_builtin_groups
if ($null -eq $WorkflowBuiltinGroups) {
    throw "generated editor metadata syntax_catalog.workflow_builtin_groups is missing"
}

function Assert-DeclarationFirstPaintFallbacks {
    $patterns = @($GrammarSource.grammar.repository.declarations.patterns | Where-Object {
        [string]$_.name -eq "meta.declaration.typed-binding.englang"
    })
    if ($patterns.Count -ne 1) {
        throw "TextMate source grammar must define exactly one typed binding pattern"
    }

    $includes = @($patterns[0].patterns | Where-Object {
        $null -ne $_.include
    } | ForEach-Object {
        [string]$_.include
    })
    $typesIndex = [array]::IndexOf($includes, "#types")
    $keywordsIndex = [array]::IndexOf($includes, "#keywords")
    if ($keywordsIndex -lt 0) {
        throw "TextMate typed bindings must include #keywords for policy words before named arguments"
    }
    if ($typesIndex -lt 0 -or $typesIndex -gt $keywordsIndex) {
        throw "TextMate typed bindings must match #types before the #keywords fallback"
    }
    $workflowOptionPatterns = @($patterns[0].patterns | Where-Object {
        [string]$_.name -eq "variable.parameter.property.englang" -and
        [string]$_.match -eq '\b({{WORKFLOW_OPTIONS}})\b(?=\s*=)'
    })
    if ($workflowOptionPatterns.Count -ne 1) {
        throw "TextMate typed bindings must color compiler-owned workflow options before named-argument equals signs"
    }

    $argsBlocks = @($GrammarSource.grammar.repository.declarations.patterns | Where-Object {
        [string]$_.name -eq "meta.declaration.args-block.englang"
    })
    if ($argsBlocks.Count -ne 1) {
        throw "TextMate source grammar must define exactly one args block pattern"
    }
    $argsIncludes = @($argsBlocks[0].patterns | Where-Object {
        $null -ne $_.include
    } | ForEach-Object {
        [string]$_.include
    })
    if ($argsIncludes -notcontains "#builtins") {
        throw "TextMate args blocks must include #builtins for default-value helper calls"
    }
}
$WorkflowBuiltinGroupNames = @(
    "deprecated", "validation", "external", "path", "temporal",
    "model", "uncertain", "timeseries", "solver", "workflow_step"
)
$WorkflowBuiltinGroupItems = @{}
$KnownWorkflowBuiltinLabels = @(($WorkflowBuiltins + $HyphenatedWorkflowBuiltins + $LegacyWorkflowBuiltinAliases) | Sort-Object -Unique)
$SeenWorkflowBuiltinGroupLabels = @{}
foreach ($WorkflowBuiltinGroupName in $WorkflowBuiltinGroupNames) {
    $GroupProperty = $WorkflowBuiltinGroups.PSObject.Properties[$WorkflowBuiltinGroupName]
    if ($null -eq $GroupProperty -or @($GroupProperty.Value).Count -eq 0) {
        throw "generated editor metadata syntax_catalog.workflow_builtin_groups.$WorkflowBuiltinGroupName is missing or empty"
    }
    $GroupLabels = @($GroupProperty.Value | ForEach-Object { [string]$_ })
    $WorkflowBuiltinGroupItems[$WorkflowBuiltinGroupName] = $GroupLabels
    foreach ($GroupLabel in $GroupLabels) {
        if ($KnownWorkflowBuiltinLabels -notcontains $GroupLabel) {
            throw "workflow builtin group $WorkflowBuiltinGroupName contains unknown label $GroupLabel"
        }
        if ($SeenWorkflowBuiltinGroupLabels.ContainsKey($GroupLabel)) {
            throw "workflow builtin label $GroupLabel appears in both $($SeenWorkflowBuiltinGroupLabels[$GroupLabel]) and $WorkflowBuiltinGroupName groups"
        }
        $SeenWorkflowBuiltinGroupLabels[$GroupLabel] = $WorkflowBuiltinGroupName
    }
}
$WorkflowOptions = @($SyntaxCatalog.workflow_options | ForEach-Object { [string]$_.label })
$LegacyWorkflowOptionAliases = @($SyntaxCatalog.legacy_workflow_option_aliases | ForEach-Object { [string]$_ })
$LanguageConstants = @($SyntaxCatalog.constants | ForEach-Object { [string]$_ })
$WorkflowStatusLiterals = @($SyntaxCatalog.workflow_status_literals | ForEach-Object { [string]$_ })
if ($WorkflowStatusLiterals.Count -eq 0) {
    throw "generated editor metadata syntax_catalog.workflow_status_literals is empty"
}
$OperatorWords = @($SyntaxCatalog.operator_words | ForEach-Object { [string]$_ })
$KeywordGroups = $SyntaxCatalog.keyword_groups
$KeywordGroupScopeChecks = @(
    @{ Name = "import"; Scope = "keyword.control.import.englang"; Labels = @($KeywordGroups.import | ForEach-Object { [string]$_ }) },
    @{ Name = "deprecated"; Scope = "keyword.control.deprecated.englang"; Labels = @($KeywordGroups.deprecated | ForEach-Object { [string]$_ }) },
    @{ Name = "declaration"; Scope = "storage.type.declaration.englang"; Labels = @($KeywordGroups.declaration | ForEach-Object { [string]$_ }) },
    @{ Name = "function"; Scope = "storage.type.function.englang"; Labels = @($KeywordGroups.function | ForEach-Object { [string]$_ }) },
    @{ Name = "test"; Scope = "storage.type.test.englang"; Labels = @($KeywordGroups.test | ForEach-Object { [string]$_ }) },
    @{ Name = "block"; Scope = "storage.type.block.englang"; Labels = @($KeywordGroups.block | ForEach-Object { [string]$_ }) },
    @{ Name = "modifier"; Scope = "storage.modifier.englang"; Labels = @($KeywordGroups.modifier | ForEach-Object { [string]$_ }) },
    @{ Name = "report"; Scope = "keyword.control.report.englang"; Labels = @($KeywordGroups.report | ForEach-Object { [string]$_ }) },
    @{ Name = "validation"; Scope = "keyword.control.validation.englang"; Labels = @($KeywordGroups.validation | ForEach-Object { [string]$_ }) },
    @{ Name = "side_effect"; Scope = "keyword.control.side-effect.englang"; Labels = @($KeywordGroups.side_effect | ForEach-Object { [string]$_ }) },
    @{ Name = "external_boundary"; Scope = "keyword.control.external-boundary.englang"; Labels = @($KeywordGroups.external_boundary | ForEach-Object { [string]$_ }) },
    @{ Name = "solver"; Scope = "keyword.control.solver.englang"; Labels = @($KeywordGroups.solver | ForEach-Object { [string]$_ }) },
    @{ Name = "workflow"; Scope = "keyword.control.workflow.englang"; Labels = @($KeywordGroups.workflow | ForEach-Object { [string]$_ }) }
)
$GrammarOnlyFunctionArgumentAliases = @(
    "axis",
    "over",
    "mean",
    "std",
    "error"
)
$GrammarWorkflowOptions = Read-GrammarWorkflowOptionLabels
$PublicTypeLabels = @($SyntaxCatalog.public_types | ForEach-Object { [string]$_.label })
$PublicGenericTypes = @($PublicTypeLabels | Where-Object {
    $_ -match "\[[^\]]+\]"
})
$PublicTypes = @($PublicTypeLabels | ForEach-Object {
    ($_ -replace "\[.*$", "")
} | Select-Object -Unique)
$CompilerUnitSymbols = @($SyntaxCatalog.units | ForEach-Object { [string]$_.label } | Select-Object -Unique)
$ContextualCompilerUnitSymbols = @($CompilerUnitSymbols | Where-Object { $_ -eq "1" })
$StandaloneCompilerUnitSymbols = @($CompilerUnitSymbols | Where-Object { $ContextualCompilerUnitSymbols -notcontains $_ })
$LegacyUnitAliases = @($SyntaxCatalog.legacy_unit_aliases | ForEach-Object { [string]$_ } | Select-Object -Unique)
$CompilerQuantityKinds = @($SyntaxCatalog.quantities | ForEach-Object { [string]$_.label } | Select-Object -Unique)
$PublicWorkflowMemberFields = @(
    $SyntaxCatalog.http_response_fields +
    $SyntaxCatalog.coverage_result_fields +
    $SyntaxCatalog.time_alignment_result_fields +
    $SyntaxCatalog.table_fields +
    $SyntaxCatalog.sample_table_fields +
    $SyntaxCatalog.db_connection_fields +
    $SyntaxCatalog.case_table_fields +
    $SyntaxCatalog.case_output_table_fields +
    $SyntaxCatalog.case_run_result_table_fields +
    $SyntaxCatalog.case_result_collection_table_fields +
    $SyntaxCatalog.model_fields +
    $SyntaxCatalog.prediction_table_fields
) | ForEach-Object { [string]$_.label } | Sort-Object -Unique
if ($PublicWorkflowMemberFields.Count -eq 0) {
    throw "generated editor metadata public workflow member field catalog is empty"
}

foreach ($selector in @("keyword.declaration", "modifier", "modifier.static")) {
    Assert-SemanticTokenScopeIncludes -Package $PackageJson -Selector $selector -Scopes @("storage.modifier.schema.englang") -Description "schema index modifier"
}

Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $CompletionKeywords -Description "LSP completion keyword"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $WorkflowBuiltins -Description "LSP workflow builtin"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $HyphenatedWorkflowBuiltins -Description "LSP hyphenated workflow builtin"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $LegacyWorkflowBuiltinAliases -Description "legacy workflow builtin alias"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $WorkflowOptions -Description "LSP workflow option"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $LegacyWorkflowOptionAliases -Description "legacy workflow option alias"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $LanguageConstants -Description "LSP language constant"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $WorkflowStatusLiterals -Description "LSP workflow status literal"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $OperatorWords -Description "LSP operator word"
foreach ($KeywordGroupCheck in $KeywordGroupScopeChecks) {
    Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $KeywordGroupCheck.Labels -Description "LSP keyword group $($KeywordGroupCheck.Name)"
}
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $PublicTypes -Description "LSP public type"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $CompilerUnitSymbols -Description "compiler unit"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $LegacyUnitAliases -Description "legacy unit alias"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $CompilerQuantityKinds -Description "compiler quantity"
Assert-GeneratedGrammarContainsLabels -Source $GrammarGeneratedRaw -Labels $PublicWorkflowMemberFields -Description "public workflow member field"
Assert-UnitsPrecedeBuiltinsInIncludeGroups -Node $GrammarSource.grammar
Assert-WorkflowOptionsAreScopedToWithBlocks
Assert-ScopeMatchesLabels -Scope "constant.language.englang" -Labels $LanguageConstants -Description "LSP language constant"
Assert-ScopeMatchesLabels -Scope "keyword.operator.word.englang" -Labels $OperatorWords -Description "LSP operator word"
foreach ($KeywordGroupCheck in $KeywordGroupScopeChecks) {
    Assert-ScopeMatchesLabels -Scope $KeywordGroupCheck.Scope -Labels $KeywordGroupCheck.Labels -Description "LSP keyword group $($KeywordGroupCheck.Name)"
}
$AllowedGrammarWorkflowOptions = @($WorkflowOptions + $LegacyWorkflowOptionAliases)
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
    "storage.modifier.state.englang",
    "storage.modifier.input.englang",
    "storage.modifier.parameter.englang",
    "storage.modifier.output.englang",
    "storage.modifier.operator.englang",
    "storage.type.test.englang",
    "storage.type.block.englang",
    "storage.type.interface-member.englang",
    "storage.modifier.schema.englang",
    "support.function.builtin.englang",
    "variable.parameter.property.englang"
)
Assert-AnyScopeMatchesLabels -Scopes $KeywordFallbackScopes -Labels $CompletionKeywords -Description "LSP completion keyword" -FixtureText $CompletionKeywordFixture
$PathWorkflowBuiltins = @($WorkflowBuiltinGroupItems["path"])
$ExternalWorkflowBuiltins = @($WorkflowBuiltinGroupItems["external"])
$TemporalWorkflowBuiltins = @($WorkflowBuiltinGroupItems["temporal"])
$ModelWorkflowBuiltins = @($WorkflowBuiltinGroupItems["model"])
$UncertaintyWorkflowBuiltins = @($WorkflowBuiltinGroupItems["uncertain"])
$TimeseriesWorkflowBuiltins = @($WorkflowBuiltinGroupItems["timeseries"])
$SolverWorkflowBuiltins = @($WorkflowBuiltinGroupItems["solver"])
$ValidationWorkflowBuiltins = @($WorkflowBuiltinGroupItems["validation"])
$DeprecatedWorkflowBuiltins = @($WorkflowBuiltinGroupItems["deprecated"])
$WorkflowStepWorkflowBuiltins = @($WorkflowBuiltinGroupItems["workflow_step"])
$DomainScopedWorkflowBuiltins = @($PathWorkflowBuiltins + $ExternalWorkflowBuiltins + $TemporalWorkflowBuiltins + $ModelWorkflowBuiltins + $UncertaintyWorkflowBuiltins + $TimeseriesWorkflowBuiltins + $SolverWorkflowBuiltins + $ValidationWorkflowBuiltins + $DeprecatedWorkflowBuiltins + $WorkflowStepWorkflowBuiltins)
$GenericWorkflowBuiltins = @($WorkflowBuiltins | Where-Object { $DomainScopedWorkflowBuiltins -notcontains $_ })
Assert-ScopeMatchesLabels -Scope "support.function.builtin.englang" -Labels $GenericWorkflowBuiltins -Description "LSP workflow builtin"
Assert-ScopeMatchesLabels -Scope "support.function.external-boundary.englang" -Labels $ExternalWorkflowBuiltins -Description "LSP external boundary builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.path.englang" -Labels $PathWorkflowBuiltins -Description "LSP path helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.temporal.englang" -Labels $TemporalWorkflowBuiltins -Description "LSP temporal helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.model.englang" -Labels $ModelWorkflowBuiltins -Description "LSP model helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.uncertain.englang" -Labels $UncertaintyWorkflowBuiltins -Description "LSP uncertainty helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.timeseries.englang" -Labels $TimeseriesWorkflowBuiltins -Description "LSP TimeSeries helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.solver.englang" -Labels $SolverWorkflowBuiltins -Description "LSP solver helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.validation.englang" -Labels $ValidationWorkflowBuiltins -Description "LSP validation helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.deprecated.englang" -Labels $DeprecatedWorkflowBuiltins -Description "LSP deprecated helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.workflow-step.englang" -Labels $WorkflowStepWorkflowBuiltins -Description "LSP workflow-step helper builtin" -Suffix "("
Assert-ScopeMatchesLabels -Scope "support.function.builtin.englang" -Labels $HyphenatedWorkflowBuiltins -Description "LSP hyphenated workflow builtin"
Assert-ScopeDoesNotMatchLabels -Scope "entity.name.function.call.englang" -Labels ($WorkflowBuiltins + $HyphenatedWorkflowBuiltins) -Description "LSP workflow builtin call" -Suffix "("
Assert-ScopeDoesNotMatchText -Scope "support.function.timeseries.englang" -Text "p42(series)" -Description "unsupported percentile helper"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.read-structured.englang" -Text 'read csv file("data/input.csv")' -Description "unsupported raw CSV read"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.validation.englang" -Text 'bad_validate = validate args.Q > 0 kW' -Description "unsupported bound validate command"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.validation.englang" -Text 'bad_assert = assert args.Q > 0 kW' -Description "unsupported bound assert command"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.validation.englang" -Text 'bad_golden = golden "summary.csv" matches file("golden/summary.csv")' -Description "unsupported bound golden command"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.summarize-series.englang" -Text 'arg_summary = summarize args.Q_total_unc by [mean, p95]' -Description "unsupported bound report summarize"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.rmse-comparison.englang" -Text 'model_error = model.rmse' -Description "unsupported model rmse member field"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.rmse-comparison.englang" -Text 'rmse_value = rmse(measured.T_zone, simulated.T_zone)' -Description "unsupported call-style rmse function"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.show-report.englang" -Text 'arg_show = show args.Q_total_unc' -Description "unsupported bound report show"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.plot-series.englang" -Text 'arg_plot_series = plot args.Q_series over args.Time' -Description "unsupported bound report plot"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.plot-distribution.englang" -Text 'arg_plot_dist = plot distribution(args.Q_dist)' -Description "unsupported bound report distribution plot"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.download-to.englang" -Text 'bad_download = download url("https://example.org/file.csv") to file("outputs/file.csv")' -Description "unsupported bound download side effect"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.export-summary-csv.englang" -Text 'bad_export = export summary to csv file("outputs/summary.csv")' -Description "unsupported bound export side effect"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.write-text.englang" -Text 'bad_write_text = write text file("outputs/out.txt"), "ok"' -Description "unsupported bound write text side effect"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.write-json.englang" -Text 'bad_write_json = write json file("outputs/out.json"), args.payload' -Description "unsupported bound write json side effect"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.write-standard-text.englang" -Text 'bad_write_standard_text = write standard_text file("outputs/out.txt"), args.weather' -Description "unsupported bound write standard text side effect"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.file-operation.englang" -Text 'bad_copy = copy file("a.txt") to file("b.txt")' -Description "unsupported bound file operation side effect"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.print-message.englang" -Text 'bad_print = print "ok"' -Description "unsupported bound print output"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.log-message.englang" -Text 'bad_log = log info "ok"' -Description "unsupported bound log output"
Assert-ScopeDoesNotMatchText -Scope "meta.block.header.englang" -Text 'bad_report = report {' -Description "unsupported bound report block"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.args-block.englang" -Text 'bad_args = args {' -Description "unsupported bound args block"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.class-object.englang" -Text 'bad_args = args {' -Description "unsupported bound args block as class object"
Assert-ScopeDoesNotMatchText -Scope "entity.name.type.declaration.englang" -Text 'bad_schema = schema Row {' -Description "unsupported bound schema declaration"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.function.englang" -Text 'bad_fn = fn helper {' -Description "unsupported bound function declaration"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.constant.englang" -Text 'bad_const = const cp' -Description "unsupported bound const declaration"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.test.englang" -Text 'bad_test = test "ok" {' -Description "unsupported bound test declaration"
Assert-ScopeDoesNotMatchText -Scope "meta.block.validation.englang" -Text 'bad_constraints = constraints {' -Description "unsupported bound validation block"
Assert-ScopeDoesNotMatchText -Scope "meta.workflow.return-statement.englang" -Text 'bad_return = return args.Q' -Description "unsupported bound return statement"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.system-member.englang" -Text 'bad_state = state T: AbsoluteTemperature [K]' -Description "unsupported bound system member declaration"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.equation.englang" -Text 'bad_equation = equation balance: Q eq load' -Description "unsupported bound equation declaration"
Assert-ScopeDoesNotMatchText -Scope "meta.declaration.interface-member.englang" -Text 'bad_port = port heat: Thermal' -Description "unsupported bound interface member declaration"
Assert-ScopeDoesNotMatchText -Scope "storage.modifier.schema.englang" -Text 'bad_index = index time' -Description "unsupported bound schema index modifier"
Assert-ScopeDoesNotMatchText -Scope "meta.report.unit-axis.englang" -Text 'bad_unit = unit y = kW' -Description "unsupported bound workflow unit option"
Assert-ScopeMatchesLabels -Scope "support.type.englang" -Labels $PublicTypes -Description "LSP public type"
Assert-ScopeMatchesLabels -Scope "meta.type.generic.englang" -Labels $PublicGenericTypes -Description "LSP public generic type"
Assert-ScopeMatchesLabels -Scope "meta.type.generic.englang" -Labels @("Array[String]", "List[Int]") -Description "schema collection generic type"
Assert-ScopeMatchesLabels -Scope "meta.type.array-suffix.englang" -Labels @("Bool[]", "String[]") -Description "schema array suffix type"
Assert-ScopeMatchesLabels -Scope "support.type.englang" -Labels $CompilerQuantityKinds -Description "compiler quantity"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.englang" -Labels $StandaloneCompilerUnitSymbols -Description "standalone compiler unit"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.format.englang" -Labels $StandaloneCompilerUnitSymbols -Description "standalone compiler unit"
if ($ContextualCompilerUnitSymbols.Count -gt 0) {
    $ContextualUnitFixture = "ratio: Ratio [1] = 0.25 1"
    Assert-AnyScopeMatchesLabels -Scopes @("constant.other.unit.englang") -Labels $ContextualCompilerUnitSymbols -Description "contextual compiler unit" -FixtureText $ContextualUnitFixture
    Assert-ScopeDoesNotMatchLabelInFixture -Scope "constant.other.unit.englang" -Label "1" -FixtureText "ratio = 1" -Description "bare numeric literal"
    Assert-ScopeDoesNotMatchLabels -Scope "constant.other.unit.format.englang" -Labels $ContextualCompilerUnitSymbols -Description "contextual compiler unit outside a unit context"
}
Assert-ScopeMatchesLabels -Scope "constant.other.unit.englang" -Labels $LegacyUnitAliases -Description "legacy unit alias"
Assert-ScopeMatchesLabels -Scope "constant.other.unit.format.englang" -Labels $LegacyUnitAliases -Description "legacy unit alias"
$PublicWorkflowMemberFixture = ($PublicWorkflowMemberFields | ForEach-Object { "api.$_" }) -join "`n"
$DottedPublicWorkflowMemberFixture = ($PublicWorkflowMemberFields | ForEach-Object { "api.resource.$_" }) -join "`n"
$ArgsPublicWorkflowMemberFixture = ($PublicWorkflowMemberFields | ForEach-Object { "args.resource.$_" }) -join "`n"
Assert-AnyScopeMatchesLabels -Scopes @("variable.other.public-member.englang") -Labels $PublicWorkflowMemberFields -Description "public workflow member field" -FixtureText $PublicWorkflowMemberFixture
Assert-AnyScopeMatchesLabels -Scopes @("variable.other.public-member.englang") -Labels $PublicWorkflowMemberFields -Description "dotted public workflow member field" -FixtureText $DottedPublicWorkflowMemberFixture
Assert-AnyScopeMatchesLabels -Scopes @("variable.other.public-member.englang") -Labels $PublicWorkflowMemberFields -Description "args dotted public workflow member field" -FixtureText $ArgsPublicWorkflowMemberFixture
Assert-ScopeDoesNotMatchLabelInFixture -Scope "constant.other.unit.englang" -Label "min" -FixtureText "min(Q_series)" -Description "function-call"
Assert-ScopeDoesNotMatchLabelInFixture -Scope "constant.other.unit.englang" -Label "min" -FixtureText "min (Q_series)" -Description "function-call"
Assert-ScopeMatchesLabels -Scope "variable.parameter.property.englang" -Labels $WorkflowOptions -Description "LSP workflow option" -Suffix " ="
Assert-ScopeMatchesLabels -Scope "variable.parameter.function.englang" -Labels ($WorkflowOptions + $LegacyWorkflowOptionAliases + $GrammarOnlyFunctionArgumentAliases) -Description "LSP workflow named argument" -Suffix "="
Assert-ExpectedTokenTextsCoverLabels -Labels $CompletionKeywords -Description "generated keyword"
Assert-ExpectedTokenTextsCoverLabels -Labels $HyphenatedWorkflowBuiltins -Description "hyphenated workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.path.englang" -Labels $PathWorkflowBuiltins -Description "path workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.external-boundary.englang" -Labels $ExternalWorkflowBuiltins -Description "external boundary workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.temporal.englang" -Labels $TemporalWorkflowBuiltins -Description "temporal workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.timeseries.englang" -Labels $TimeseriesWorkflowBuiltins -Description "TimeSeries workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.solver.englang" -Labels $SolverWorkflowBuiltins -Description "solver workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.validation.englang" -Labels $ValidationWorkflowBuiltins -Description "validation workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.deprecated.englang" -Labels $DeprecatedWorkflowBuiltins -Description "deprecated workflow builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "support.function.workflow-step.englang" -Labels $WorkflowStepWorkflowBuiltins -Description "workflow-step builtin"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "constant.language.englang" -Labels $LanguageConstants -Description "language constant"
Assert-ExpectedScopedTokenTextsCoverLabels -Scope "keyword.operator.word.englang" -Labels $OperatorWords -Description "operator word"
foreach ($KeywordGroupCheck in $KeywordGroupScopeChecks) {
    Assert-ExpectedScopedTokenTextsCoverLabels -Scope $KeywordGroupCheck.Scope -Labels $KeywordGroupCheck.Labels -Description "keyword group $($KeywordGroupCheck.Name)"
}
Assert-ExpectedWorkflowScopesCoverGrammar
Assert-ExpectedLeafScopesCoverGrammar
Assert-ActualWorkflowPublicMemberExpectedTokens
Assert-BundledThemeLeafScopeCoverage
Assert-WorkflowPatternIncludes -Name "meta.workflow.integrate-series.englang" -Include "#members" -Description "integrate series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.stat-series.englang" -Include "#members" -Description "stat series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.rmse-comparison.englang" -Include "#members" -Description "rmse comparison"
Assert-WorkflowPatternIncludes -Name "meta.workflow.check-coverage.englang" -Include "#members" -Description "check coverage"
Assert-WorkflowPatternIncludes -Name "meta.workflow.fill-missing.englang" -Include "#members" -Description "fill missing"
Assert-WorkflowPatternIncludes -Name "meta.workflow.align-series.englang" -Include "#members" -Description "align series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.resample-series.englang" -Include "#members" -Description "resample series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.render-template.englang" -Include "#operators" -Description "render template"
Assert-WorkflowPatternIncludes -Name "meta.workflow.promote-csv.englang" -Include "#members" -Description "promote csv"
Assert-WorkflowPatternIncludes -Name "meta.workflow.promote-json.englang" -Include "#members" -Description "promote json"
Assert-WorkflowPatternIncludes -Name "meta.workflow.promote-toml.englang" -Include "#members" -Description "promote toml"
Assert-WorkflowPatternIncludes -Name "meta.workflow.promote-json-records.englang" -Include "#members" -Description "promote json records"
Assert-WorkflowPatternIncludes -Name "meta.workflow.plot-distribution.englang" -Include "#members" -Description "plot distribution"
Assert-WorkflowPatternIncludes -Name "meta.workflow.plot-series.englang" -Include "#members" -Description "plot series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.plot-command.englang" -Include "#members" -Description "plot command"
Assert-WorkflowPatternIncludes -Name "meta.workflow.download-to.englang" -Include "#operators" -Description "download"
Assert-WorkflowPatternIncludes -Name "meta.workflow.print-message.englang" -Include "#operators" -Description "print message"
Assert-WorkflowPatternIncludes -Name "meta.workflow.print-message.englang" -Include "#members" -Description "print message"
Assert-WorkflowPatternIncludes -Name "meta.workflow.log-message.englang" -Include "#operators" -Description "log message"
Assert-WorkflowPatternIncludes -Name "meta.workflow.log-message.englang" -Include "#members" -Description "log message"
Assert-WorkflowPatternIncludes -Name "meta.workflow.write-standard-text.englang" -Include "#operators" -Description "write standard text"
Assert-WorkflowPatternIncludes -Name "meta.workflow.run-command.englang" -Include "#operators" -Description "run command"
Assert-WorkflowPatternIncludes -Name "meta.workflow.run-command.englang" -Include "#members" -Description "run command"
Assert-WorkflowPatternIncludes -Name "meta.workflow.summarize-series.englang" -Include "#operators" -Description "summarize series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.summarize-series.englang" -Include "#members" -Description "summarize series"
Assert-WorkflowPatternIncludes -Name "meta.workflow.show-report.englang" -Include "#members" -Description "show report"
Assert-WorkflowPatternIncludes -Name "meta.workflow.http-request.englang" -Include "#operators" -Description "http request"
Assert-WorkflowPatternIncludes -Name "meta.workflow.export-summary-csv.englang" -Include "#members" -Description "export summary csv"
Assert-WorkflowPatternIncludes -Name "meta.workflow.write-text.englang" -Include "#members" -Description "write text"
Assert-WorkflowPatternIncludes -Name "meta.workflow.write-json.englang" -Include "#members" -Description "write json"
Assert-WorkflowPatternIncludes -Name "meta.workflow.write-standard-text.englang" -Include "#members" -Description "write standard text"
Assert-WorkflowBeginCaptureScope -Name "meta.workflow.write-text.englang" -Begin "^\s*(write)\s+(text)\b" -CaptureIndex "2" -Scope "keyword.control.side-effect.englang" -Description "write text format"
Assert-WorkflowBeginCaptureScope -Name "meta.workflow.write-json.englang" -Begin "^\s*(write)\s+(json)\b" -CaptureIndex "2" -Scope "keyword.control.side-effect.englang" -Description "write json format"
Assert-WorkflowBeginCaptureScope -Name "meta.workflow.write-standard-text.englang" -Begin "^\s*(write)\s+(standard_text)\b" -CaptureIndex "2" -Scope "keyword.control.side-effect.englang" -Description "write standard text format"
Assert-WorkflowPatternIncludes -Name "meta.workflow.file-operation.englang" -Include "#members" -Description "file operation"
Assert-WorkflowPatternIncludes -Name "meta.workflow.render-template.englang" -Include "#members" -Description "render template"
Assert-WorkflowPatternIncludes -Name "meta.workflow.read-structured.englang" -Include "#members" -Description "read structured"
Assert-WorkflowPatternIncludes -Name "meta.workflow.download-to.englang" -Include "#members" -Description "download"
Assert-WorkflowPatternIncludes -Name "meta.workflow.http-request.englang" -Include "#members" -Description "http request"
Assert-WorkflowPatternIncludes -Name "meta.workflow.db-read.englang" -Include "#members" -Description "db read"
Assert-WorkflowPatternIncludes -Name "meta.workflow.open-sqlite.englang" -Include "#members" -Description "open sqlite"
Assert-WorkflowPatternIncludes -Name "meta.workflow.db-write.englang" -Include "#members" -Description "db write"
Assert-WorkflowPatternIncludes -Name "meta.workflow.select-columns.englang" -Include "#members" -Description "select columns"
Assert-WorkflowPatternIncludes -Name "meta.workflow.model-train-call.englang" -Include "#members" -Description "model train call"
Assert-WorkflowPatternIncludes -Name "meta.workflow.regression-table.englang" -Include "#members" -Description "regression table"
Assert-WorkflowPatternIncludes -Name "meta.workflow.model-summary-call.englang" -Include "#members" -Description "model summary call"
Assert-WorkflowPatternIncludes -Name "meta.workflow.distribution-call.englang" -Include "#members" -Description "distribution call"
Assert-WorkflowPatternIncludes -Name "meta.workflow.filter-table.englang" -Include "#members" -Description "filter table"
Assert-WorkflowPatternIncludes -Name "meta.workflow.derive-column.englang" -Include "#members" -Description "derive column"
Assert-WorkflowPatternIncludes -Name "meta.workflow.sort-table.englang" -Include "#members" -Description "sort table"
Assert-WorkflowPatternIncludes -Name "meta.workflow.join-table.englang" -Include "#members" -Description "join table"
Assert-WorkflowPatternIncludes -Name "meta.workflow.materialize-cases.englang" -Include "#members" -Description "materialize cases"
Assert-WorkflowPatternIncludes -Name "meta.workflow.collect-results.englang" -Include "#members" -Description "collect results"
Assert-WorkflowPatternIncludes -Name "meta.workflow.require-one.englang" -Include "#members" -Description "require one"
Assert-WorkflowPatternIncludes -Name "meta.workflow.predict-model.englang" -Include "#members" -Description "predict model"
Assert-WorkflowPatternIncludes -Name "meta.workflow.train-regression.englang" -Include "#members" -Description "train regression"
Assert-WorkflowPatternIncludes -Name "meta.workflow.apply-call.englang" -Include "#members" -Description "apply call"
Assert-WorkflowPatternIncludes -Name "meta.workflow.apply-step.englang" -Include "#members" -Description "apply step"
Assert-WorkflowPatternIncludes -Name "meta.workflow.integrate-call.englang" -Include "#members" -Description "integrate call"
Assert-WorkflowPatternIncludes -Name "meta.workflow.stat-axis-call.englang" -Include "#members" -Description "stat axis call"
Assert-WorkflowPatternIncludes -Name "meta.workflow.summary-field.englang" -Include "#members" -Description "summary field"
Assert-BeginEndWorkflowPhrasesAreMemberAware
Assert-WorkflowPropertyFallbacksAreMemberAware
Assert-FunctionCallFallbacks
Assert-DeclarationFirstPaintFallbacks
Assert-MemberPathFallbackOrder
Assert-WorkflowStatusOptionPattern
Assert-WithBlockExpressionFallbacks

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

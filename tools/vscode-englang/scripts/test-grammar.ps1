param(
    [string] $ExtensionRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [string] $OutputRoot = ""
)

$ErrorActionPreference = "Stop"

if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $ExtensionRoot)
    $OutputRoot = Join-Path $RepoRoot "build\editor-tests\textmate_tokens"
}

& (Join-Path $PSScriptRoot "build-grammar.ps1") -ExtensionRoot $ExtensionRoot -Check

$GrammarPath = Join-Path $ExtensionRoot "syntaxes\eng.tmLanguage.json"
$ExpectedPath = Join-Path $ExtensionRoot "test\expected\grammar_tokens.json"
$FixtureRoot = Join-Path $ExtensionRoot "test\grammar-fixtures"
foreach ($RequiredPath in @($GrammarPath, $ExpectedPath, $FixtureRoot)) {
    if (-not (Test-Path -LiteralPath $RequiredPath)) {
        throw "missing grammar test input at $RequiredPath"
    }
}

$Grammar = Get-Content -LiteralPath $GrammarPath -Raw -Encoding UTF8 | ConvertFrom-Json
$ExpectedJson = Get-Content -LiteralPath $ExpectedPath -Raw -Encoding UTF8 | ConvertFrom-Json
$Expected = New-Object System.Collections.Generic.List[object]
if ($ExpectedJson -is [System.Array]) {
    foreach ($item in $ExpectedJson) {
        $Expected.Add($item) | Out-Null
    }
} else {
    $Expected.Add($ExpectedJson) | Out-Null
}
$PatternsByScope = @{}

function Add-PatternNode {
    param([object] $Node)

    if ($null -eq $Node) {
        return
    }
    if ($null -ne $Node.name) {
        $scope = [string] $Node.name
        if (-not $PatternsByScope.ContainsKey($scope)) {
            $PatternsByScope[$scope] = New-Object System.Collections.Generic.List[object]
        }
        $PatternsByScope[$scope].Add($Node) | Out-Null
    }
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
        [Parameter(Mandatory = $true)][string] $Text
    )

    if ($null -ne $Pattern.match) {
        return [regex]::IsMatch($Text, [string] $Pattern.match)
    }
    if ($null -ne $Pattern.begin -and $null -ne $Pattern.end) {
        return [regex]::IsMatch($Text, [string] $Pattern.begin) -and [regex]::IsMatch($Text, [string] $Pattern.end)
    }
    return $false
}

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
    $matched = $false
    foreach ($pattern in $PatternsByScope[[string] $case.scope]) {
        if (Test-PatternMatchesText -Pattern $pattern -Text ([string] $case.text)) {
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

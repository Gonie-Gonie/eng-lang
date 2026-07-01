param(
    [string] $ExtensionRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [switch] $Check
)

$ErrorActionPreference = "Stop"

$SourcePath = Join-Path $ExtensionRoot "syntaxes\eng.tmLanguage.source.json"
if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) {
    throw "missing TextMate grammar source at $SourcePath"
}

$Source = Get-Content -LiteralPath $SourcePath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($null -eq $Source.grammar) {
    throw "TextMate grammar source must contain a grammar object"
}
if ([string]::IsNullOrWhiteSpace($Source.generatedPath)) {
    throw "TextMate grammar source must contain generatedPath"
}

$GeneratedPath = Join-Path (Split-Path -Parent $SourcePath) $Source.generatedPath
$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
$Generated = (($Source.grammar | ConvertTo-Json -Depth 64) -replace "`r`n", "`n") + "`n"

if ($Check) {
    if (-not (Test-Path -LiteralPath $GeneratedPath -PathType Leaf)) {
        throw "generated TextMate grammar is missing at $GeneratedPath"
    }
    $Current = Get-Content -LiteralPath $GeneratedPath -Raw -Encoding UTF8
    $Current = $Current -replace "`r`n", "`n"
    if ($Current -ne $Generated) {
        throw "generated TextMate grammar is out of sync. Run .\dev.bat vscode-build-grammar"
    }
    Write-Host "VS Code TextMate grammar is in sync."
    exit 0
}

[System.IO.File]::WriteAllText($GeneratedPath, $Generated, $Utf8NoBom)
Write-Host "Generated VS Code TextMate grammar at $GeneratedPath"

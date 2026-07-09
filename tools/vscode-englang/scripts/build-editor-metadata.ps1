param(
    [string] $ExtensionRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [switch] $Check
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent (Split-Path -Parent $ExtensionRoot)
$OutputRoot = Join-Path $ExtensionRoot "generated\editor"
$FullMetadataPath = Join-Path $OutputRoot "englang-editor-metadata.json"
$SemanticLegendPath = Join-Path $OutputRoot "englang-semantic-legend.json"
$CompletionsPath = Join-Path $OutputRoot "englang-completions.json"
$SyntaxCatalogPath = Join-Path $OutputRoot "englang-syntax.json"
$Cargo = Get-Command cargo -ErrorAction SilentlyContinue
if ($null -eq $Cargo) {
    throw "Cargo not found. Run .\dev.bat setup."
}

Push-Location $RepoRoot
try {
    $MetadataOutput = & $Cargo.Source "run" "-p" "eng_lsp" "--quiet" "--" "--editor-metadata"
    if ($LASTEXITCODE -ne 0) {
        throw "eng-lsp --editor-metadata failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$MetadataText = ($MetadataOutput | Out-String).Trim()
$Metadata = $MetadataText | ConvertFrom-Json
if ($Metadata.format -ne "eng-lsp-editor-metadata-v1") {
    throw "eng-lsp --editor-metadata returned unexpected format $($Metadata.format)"
}

function ConvertTo-StableJson {
    param([Parameter(Mandatory = $true)][object] $Value)

    (($Value | ConvertTo-Json -Depth 64) -replace "`r`n", "`n") + "`n"
}

$SemanticLegend = [ordered]@{
    format = $Metadata.format
    semantic_token_legend = $Metadata.semantic_token_legend
}
$Completions = [ordered]@{
    format = $Metadata.format
    completion_items_count = $Metadata.completion_items_count
    completion_items = $Metadata.completion_items
    completion_seed_count = $Metadata.completion_seed_count
    completion_seed = $Metadata.completion_seed
}
$SyntaxCatalog = [ordered]@{
    format = $Metadata.format
    syntax_catalog = $Metadata.syntax_catalog
}

$ExpectedFiles = @(
    @{ Path = $FullMetadataPath; Content = ConvertTo-StableJson $Metadata },
    @{ Path = $SemanticLegendPath; Content = ConvertTo-StableJson $SemanticLegend },
    @{ Path = $CompletionsPath; Content = ConvertTo-StableJson $Completions },
    @{ Path = $SyntaxCatalogPath; Content = ConvertTo-StableJson $SyntaxCatalog }
)

if ($Check) {
    foreach ($File in $ExpectedFiles) {
        if (-not (Test-Path -LiteralPath $File.Path -PathType Leaf)) {
            throw "generated editor metadata is missing at $($File.Path). Run .\dev.bat vscode-build-editor-metadata"
        }
        $Current = Get-Content -LiteralPath $File.Path -Raw -Encoding UTF8
        $Current = $Current -replace "`r`n", "`n"
        if ($Current -ne $File.Content) {
            throw "generated editor metadata is out of sync at $($File.Path). Run .\dev.bat vscode-build-editor-metadata"
        }
    }
    Write-Host "VS Code editor metadata is in sync."
    exit 0
}

$Utf8NoBom = New-Object System.Text.UTF8Encoding($false)
New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
foreach ($File in $ExpectedFiles) {
    [System.IO.File]::WriteAllText($File.Path, $File.Content, $Utf8NoBom)
}
Write-Host "Generated VS Code editor metadata under $OutputRoot"

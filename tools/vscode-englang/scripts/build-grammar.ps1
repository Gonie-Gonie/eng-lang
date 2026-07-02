param(
    [string] $ExtensionRoot = (Split-Path -Parent (Split-Path -Parent $PSScriptRoot)),
    [switch] $Check
)

$ErrorActionPreference = "Stop"

$SourcePath = Join-Path $ExtensionRoot "syntaxes\eng.tmLanguage.source.json"
if (-not (Test-Path -LiteralPath $SourcePath -PathType Leaf)) {
    throw "missing TextMate grammar source at $SourcePath"
}
$EditorMetadataPath = Join-Path $ExtensionRoot "generated\editor\englang-editor-metadata.json"
if (-not (Test-Path -LiteralPath $EditorMetadataPath -PathType Leaf)) {
    throw "missing generated editor metadata at $EditorMetadataPath. Run .\dev.bat vscode-build-editor-metadata"
}

function ConvertTo-RegexAlternation {
    param([Parameter(Mandatory = $true)][string[]] $Labels)

    $UniqueLabels = @($Labels | Where-Object { -not [string]::IsNullOrWhiteSpace($_) } | Sort-Object -Unique)
    $SortedLabels = @($UniqueLabels | Sort-Object @{ Expression = { $_.Length }; Descending = $true }, @{ Expression = { $_ }; Ascending = $true })
    return ($SortedLabels | ForEach-Object { [regex]::Escape($_) }) -join "|"
}

function ConvertTo-JsonRegexFragment {
    param([Parameter(Mandatory = $true)][string] $Value)

    return $Value.Replace("\", "\\").Replace('"', '\"')
}

function Expand-GrammarTemplates {
    param(
        [Parameter(Mandatory = $true)][string] $Source,
        [Parameter(Mandatory = $true)][hashtable] $Templates
    )

    $Expanded = $Source
    foreach ($Template in $Templates.GetEnumerator()) {
        $Expanded = $Expanded.Replace($Template.Key, (ConvertTo-JsonRegexFragment $Template.Value))
    }
    if ($Expanded -match "\{\{[A-Z0-9_]+\}\}") {
        throw "TextMate grammar source contains unresolved template $($Matches[0])"
    }
    return $Expanded
}

$EditorMetadata = Get-Content -LiteralPath $EditorMetadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
$SyntaxCatalog = $EditorMetadata.syntax_catalog
if ($null -eq $SyntaxCatalog) {
    throw "generated editor metadata is missing syntax_catalog. Run .\dev.bat vscode-build-editor-metadata"
}
$WorkflowBuiltins = @($SyntaxCatalog.workflow_builtins | ForEach-Object { [string]$_ })
$WorkflowOptions = @($SyntaxCatalog.workflow_options | ForEach-Object { [string]$_.label })
# Keep legacy option keys colored for existing files without suggesting them
# through the generated completion catalog.
$GrammarOnlyWorkflowOptionAliases = @(
    "fixture"
)
# Preserve TextMate-only aliases until the compiler-owned catalog exposes these
# artifact, byte-size, and compatibility quantity labels directly.
$GrammarOnlyTypeAliases = @(
    "ArtifactManifest",
    "CacheManifest",
    "CaseManifest",
    "CaseTable",
    "CoverageResult",
    "DbWriteManifest",
    "HttpResponse",
    "NetworkCache",
    "Object",
    "OutputManifest",
    "QualityResult",
    "ReviewDocument",
    "RunLock",
    "RunLog",
    "RunPlan",
    "SampleTable",
    "TableRow",
    "Time",
    "PredictionManifest",
    "Area",
    "Volume",
    "Mass"
)
$GrammarOnlyUnitAliases = @(
    "B",
    "byte",
    "bytes",
    "KB",
    "kilobyte",
    "kilobytes",
    "MB",
    "megabyte",
    "megabytes",
    "GB",
    "gigabyte",
    "gigabytes",
    "KiB",
    "kibibyte",
    "kibibytes",
    "MiB",
    "mebibyte",
    "mebibytes",
    "GiB",
    "gibibyte",
    "gibibytes",
    "m2",
    "m3",
    "kJ",
    "%"
)
$PublicTypeBases = @($SyntaxCatalog.public_types | ForEach-Object { [string]$_.base }) + $GrammarOnlyTypeAliases
$QuantityLabels = @($SyntaxCatalog.quantities | ForEach-Object { [string]$_.label })
$AsciiUnits = @($SyntaxCatalog.units | ForEach-Object { [string]$_.label } | Where-Object {
    $_ -cmatch '^[\x20-\x7E]+$'
}) + $GrammarOnlyUnitAliases
$TemplateValues = @{
    "{{ASCII_UNITS}}" = ConvertTo-RegexAlternation $AsciiUnits
    "{{PUBLIC_TYPE_BASES}}" = ConvertTo-RegexAlternation $PublicTypeBases
    "{{QUANTITY_LABELS}}" = ConvertTo-RegexAlternation $QuantityLabels
    "{{WORKFLOW_BUILTINS}}" = ConvertTo-RegexAlternation $WorkflowBuiltins
    "{{WORKFLOW_OPTIONS}}" = ConvertTo-RegexAlternation ($WorkflowOptions + $GrammarOnlyWorkflowOptionAliases)
}

$SourceRaw = Get-Content -LiteralPath $SourcePath -Raw -Encoding UTF8
$Source = (Expand-GrammarTemplates -Source $SourceRaw -Templates $TemplateValues) | ConvertFrom-Json
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

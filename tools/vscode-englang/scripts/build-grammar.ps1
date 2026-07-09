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

function Assert-SyntaxCatalogArray {
    param(
        [Parameter(Mandatory = $true)][object] $Catalog,
        [Parameter(Mandatory = $true)][string] $Name
    )

    $property = $Catalog.PSObject.Properties[$Name]
    if ($null -eq $property -or $null -eq $property.Value) {
        throw "generated editor metadata syntax_catalog is missing $Name. Run .\dev.bat vscode-build-editor-metadata"
    }
    $items = @($property.Value)
    if ($items.Count -eq 0) {
        throw "generated editor metadata syntax_catalog.$Name is empty"
    }
    return $items
}

function Assert-CatalogItemsHaveProperty {
    param(
        [Parameter(Mandatory = $true)][object[]] $Items,
        [Parameter(Mandatory = $true)][string] $CatalogName,
        [Parameter(Mandatory = $true)][string] $PropertyName
    )

    foreach ($item in $Items) {
        $property = $item.PSObject.Properties[$PropertyName]
        if ($null -eq $property -or [string]::IsNullOrWhiteSpace([string]$property.Value)) {
            throw "generated editor metadata syntax_catalog.$CatalogName item is missing $PropertyName"
        }
    }
}

$EditorMetadata = Get-Content -LiteralPath $EditorMetadataPath -Raw -Encoding UTF8 | ConvertFrom-Json
$SyntaxCatalog = $EditorMetadata.syntax_catalog
if ($null -eq $SyntaxCatalog) {
    throw "generated editor metadata is missing syntax_catalog. Run .\dev.bat vscode-build-editor-metadata"
}
$KeywordItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "keywords"
$WorkflowBuiltinItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "workflow_builtins"
$HyphenatedWorkflowBuiltinItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "hyphenated_workflow_builtins"
$WorkflowOptionItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "workflow_options"
$LanguageConstantItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "constants"
$OperatorWordItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "operator_words"
$PublicTypeItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "public_types"
$QuantityItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "quantities"
$UnitItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "units"
Assert-CatalogItemsHaveProperty -Items $WorkflowOptionItems -CatalogName "workflow_options" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $PublicTypeItems -CatalogName "public_types" -PropertyName "base"
Assert-CatalogItemsHaveProperty -Items $QuantityItems -CatalogName "quantities" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $UnitItems -CatalogName "units" -PropertyName "label"
$WorkflowBuiltins = @($WorkflowBuiltinItems | ForEach-Object { [string]$_ })
$HyphenatedWorkflowBuiltins = @($HyphenatedWorkflowBuiltinItems | ForEach-Object { [string]$_ })
$WorkflowOptions = @($WorkflowOptionItems | ForEach-Object { [string]$_.label })
$LanguageConstants = @($LanguageConstantItems | ForEach-Object { [string]$_ })
$OperatorWords = @($OperatorWordItems | ForEach-Object { [string]$_ })
# Keep legacy workflow helper spellings colored for existing files without
# suggesting them through the generated completion catalog.
$GrammarOnlyWorkflowBuiltinAliases = @(
    "regression_table",
    "train_regression"
)
# Keep legacy option keys colored for existing files without suggesting them
# through the generated completion catalog.
$GrammarOnlyWorkflowOptionAliases = @(
    "fixture"
)
$GrammarOnlyFunctionArgumentAliases = @(
    "axis",
    "over",
    "mean",
    "std",
    "error"
)
# Preserve broad TextMate operator coloring for legacy clause words while the
# compiler-owned operator catalog stays focused on expression/operator words.
$GrammarOnlyOperatorWordAliases = @(
    "none",
    "null",
    "from",
    "on",
    "with",
    "where"
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
$PublicTypeBases = @($PublicTypeItems | ForEach-Object { [string]$_.base }) + $GrammarOnlyTypeAliases
$QuantityLabels = @($QuantityItems | ForEach-Object { [string]$_.label })
$AsciiUnits = @($UnitItems | ForEach-Object { [string]$_.label } | Where-Object {
    $_ -cmatch '^[\x20-\x7E]+$'
}) + $GrammarOnlyUnitAliases
$TemplateValues = @{
    "{{ASCII_UNITS}}" = ConvertTo-RegexAlternation $AsciiUnits
    "{{LANGUAGE_CONSTANTS}}" = ConvertTo-RegexAlternation $LanguageConstants
    "{{OPERATOR_WORDS}}" = ConvertTo-RegexAlternation ($OperatorWords + $GrammarOnlyOperatorWordAliases)
    "{{PUBLIC_TYPE_BASES}}" = ConvertTo-RegexAlternation $PublicTypeBases
    "{{QUANTITY_LABELS}}" = ConvertTo-RegexAlternation $QuantityLabels
    "{{WORKFLOW_BUILTINS}}" = ConvertTo-RegexAlternation ($WorkflowBuiltins + $HyphenatedWorkflowBuiltins + $GrammarOnlyWorkflowBuiltinAliases)
    "{{WORKFLOW_OPTIONS}}" = ConvertTo-RegexAlternation ($WorkflowOptions + $GrammarOnlyWorkflowOptionAliases)
    "{{WORKFLOW_NAMED_ARGS}}" = ConvertTo-RegexAlternation ($WorkflowOptions + $GrammarOnlyWorkflowOptionAliases + $GrammarOnlyFunctionArgumentAliases)
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

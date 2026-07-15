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
$KeywordGroups = $SyntaxCatalog.keyword_groups
if ($null -eq $KeywordGroups) {
    throw "generated editor metadata syntax_catalog is missing keyword_groups. Run .\dev.bat vscode-build-editor-metadata"
}
$KeywordGroupNames = @(
    "import",
    "deprecated",
    "declaration",
    "function",
    "test",
    "block",
    "modifier",
    "report",
    "validation",
    "side_effect",
    "external_boundary",
    "solver",
    "workflow"
)
$KeywordGroupItems = @{}
foreach ($KeywordGroupName in $KeywordGroupNames) {
    $KeywordGroupItems[$KeywordGroupName] = Assert-SyntaxCatalogArray -Catalog $KeywordGroups -Name $KeywordGroupName
}
$WorkflowBuiltinGroups = $SyntaxCatalog.workflow_builtin_groups
if ($null -eq $WorkflowBuiltinGroups) {
    throw "generated editor metadata syntax_catalog is missing workflow_builtin_groups. Run .\dev.bat vscode-build-editor-metadata"
}
$WorkflowBuiltinGroupNames = @(
    "deprecated",
    "validation",
    "external",
    "path",
    "temporal",
    "model",
    "uncertain",
    "timeseries",
    "solver",
    "workflow_step"
)
$WorkflowBuiltinGroupItems = @{}
foreach ($WorkflowBuiltinGroupName in $WorkflowBuiltinGroupNames) {
    $WorkflowBuiltinGroupItems[$WorkflowBuiltinGroupName] = Assert-SyntaxCatalogArray -Catalog $WorkflowBuiltinGroups -Name $WorkflowBuiltinGroupName
}
$KeywordItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "keywords"
$WorkflowBuiltinItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "workflow_builtins"
$HyphenatedWorkflowBuiltinItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "hyphenated_workflow_builtins"
$LegacyWorkflowBuiltinAliasItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "legacy_workflow_builtin_aliases"
$WorkflowOptionItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "workflow_options"
$LegacyWorkflowOptionAliasItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "legacy_workflow_option_aliases"
$LanguageConstantItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "constants"
$WorkflowStatusLiteralItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "workflow_status_literals"
$OperatorWordItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "operator_words"
$PublicTypeItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "public_types"
$QuantityItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "quantities"
$UnitItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "units"
$LegacyUnitAliasItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "legacy_unit_aliases"
$HttpResponseFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "http_response_fields"
$CoverageResultFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "coverage_result_fields"
$TableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "table_fields"
$SampleTableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "sample_table_fields"
$DbConnectionFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "db_connection_fields"
$CaseTableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "case_table_fields"
$CaseOutputTableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "case_output_table_fields"
$CaseRunResultTableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "case_run_result_table_fields"
$CaseResultCollectionTableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "case_result_collection_table_fields"
$ModelFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "model_fields"
$PredictionTableFieldItems = Assert-SyntaxCatalogArray -Catalog $SyntaxCatalog -Name "prediction_table_fields"
Assert-CatalogItemsHaveProperty -Items $WorkflowOptionItems -CatalogName "workflow_options" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $PublicTypeItems -CatalogName "public_types" -PropertyName "base"
Assert-CatalogItemsHaveProperty -Items $QuantityItems -CatalogName "quantities" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $UnitItems -CatalogName "units" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $HttpResponseFieldItems -CatalogName "http_response_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $CoverageResultFieldItems -CatalogName "coverage_result_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $TableFieldItems -CatalogName "table_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $SampleTableFieldItems -CatalogName "sample_table_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $DbConnectionFieldItems -CatalogName "db_connection_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $CaseTableFieldItems -CatalogName "case_table_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $CaseOutputTableFieldItems -CatalogName "case_output_table_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $CaseRunResultTableFieldItems -CatalogName "case_run_result_table_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $CaseResultCollectionTableFieldItems -CatalogName "case_result_collection_table_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $ModelFieldItems -CatalogName "model_fields" -PropertyName "label"
Assert-CatalogItemsHaveProperty -Items $PredictionTableFieldItems -CatalogName "prediction_table_fields" -PropertyName "label"
$WorkflowBuiltins = @($WorkflowBuiltinItems | ForEach-Object { [string]$_ })
$HyphenatedWorkflowBuiltins = @($HyphenatedWorkflowBuiltinItems | ForEach-Object { [string]$_ })
$LegacyWorkflowBuiltinAliases = @($LegacyWorkflowBuiltinAliasItems | ForEach-Object { [string]$_ })
$WorkflowOptions = @($WorkflowOptionItems | ForEach-Object { [string]$_.label })
$LegacyWorkflowOptionAliases = @($LegacyWorkflowOptionAliasItems | ForEach-Object { [string]$_ })
$LanguageConstants = @($LanguageConstantItems | ForEach-Object { [string]$_ })
$WorkflowStatusLiterals = @($WorkflowStatusLiteralItems | ForEach-Object { [string]$_ })
$OperatorWords = @($OperatorWordItems | ForEach-Object { [string]$_ })
$LegacyUnitAliases = @($LegacyUnitAliasItems | ForEach-Object { [string]$_ })
$PublicMemberFields = @(
    $HttpResponseFieldItems +
    $CoverageResultFieldItems +
    $TableFieldItems +
    $SampleTableFieldItems +
    $DbConnectionFieldItems +
    $CaseTableFieldItems +
    $CaseOutputTableFieldItems +
    $CaseRunResultTableFieldItems +
    $CaseResultCollectionTableFieldItems +
    $ModelFieldItems +
    $PredictionTableFieldItems
) | ForEach-Object { [string]$_.label } | Sort-Object -Unique

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

$PublicTypeBases = @($PublicTypeItems | ForEach-Object { [string]$_.base }) + $GrammarOnlyTypeAliases
$QuantityLabels = @($QuantityItems | ForEach-Object { [string]$_.label })
$UnitLabels = @($UnitItems | ForEach-Object { [string]$_.label }) + $LegacyUnitAliases
$TimeseriesStatWorkflowBuiltins = @($WorkflowBuiltinGroupItems["timeseries"] | Where-Object { [string]$_ -ne "integrate" })
$TemplateValues = @{
    "{{UNIT_LABELS}}" = ConvertTo-RegexAlternation $UnitLabels
    "{{LANGUAGE_CONSTANTS}}" = ConvertTo-RegexAlternation $LanguageConstants
    "{{WORKFLOW_STATUS_LITERALS}}" = ConvertTo-RegexAlternation $WorkflowStatusLiterals
    "{{OPERATOR_WORDS}}" = ConvertTo-RegexAlternation ($OperatorWords + $GrammarOnlyOperatorWordAliases)
    "{{KEYWORD_GROUP_IMPORT}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["import"])
    "{{KEYWORD_GROUP_DEPRECATED}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["deprecated"])
    "{{KEYWORD_GROUP_DECLARATION}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["declaration"])
    "{{KEYWORD_GROUP_FUNCTION}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["function"])
    "{{KEYWORD_GROUP_TEST}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["test"])
    "{{KEYWORD_GROUP_BLOCK}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["block"])
    "{{KEYWORD_GROUP_MODIFIER}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["modifier"])
    "{{KEYWORD_GROUP_REPORT}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["report"])
    "{{KEYWORD_GROUP_VALIDATION}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["validation"])
    "{{KEYWORD_GROUP_SIDE_EFFECT}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["side_effect"])
    "{{KEYWORD_GROUP_EXTERNAL_BOUNDARY}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["external_boundary"])
    "{{KEYWORD_GROUP_SOLVER}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["solver"])
    "{{KEYWORD_GROUP_WORKFLOW}}" = ConvertTo-RegexAlternation @($KeywordGroupItems["workflow"])
    "{{WORKFLOW_BUILTIN_GROUP_DEPRECATED}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["deprecated"])
    "{{WORKFLOW_BUILTIN_GROUP_VALIDATION}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["validation"])
    "{{WORKFLOW_BUILTIN_GROUP_EXTERNAL}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["external"])
    "{{WORKFLOW_BUILTIN_GROUP_PATH}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["path"])
    "{{WORKFLOW_BUILTIN_GROUP_TEMPORAL}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["temporal"])
    "{{WORKFLOW_BUILTIN_GROUP_MODEL}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["model"])
    "{{WORKFLOW_BUILTIN_GROUP_UNCERTAIN}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["uncertain"])
    "{{WORKFLOW_BUILTIN_GROUP_TIMESERIES}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["timeseries"])
    "{{WORKFLOW_BUILTIN_GROUP_TIMESERIES_STATS}}" = ConvertTo-RegexAlternation $TimeseriesStatWorkflowBuiltins
    "{{WORKFLOW_BUILTIN_GROUP_SOLVER}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["solver"])
    "{{WORKFLOW_BUILTIN_GROUP_WORKFLOW_STEP}}" = ConvertTo-RegexAlternation @($WorkflowBuiltinGroupItems["workflow_step"])
    "{{PUBLIC_TYPE_BASES}}" = ConvertTo-RegexAlternation $PublicTypeBases
    "{{QUANTITY_LABELS}}" = ConvertTo-RegexAlternation $QuantityLabels
    "{{WORKFLOW_BUILTINS}}" = ConvertTo-RegexAlternation ($WorkflowBuiltins + $HyphenatedWorkflowBuiltins + $LegacyWorkflowBuiltinAliases)
    "{{WORKFLOW_OPTIONS}}" = ConvertTo-RegexAlternation ($WorkflowOptions + $LegacyWorkflowOptionAliases)
    "{{WORKFLOW_NAMED_ARGS}}" = ConvertTo-RegexAlternation ($WorkflowOptions + $LegacyWorkflowOptionAliases + $GrammarOnlyFunctionArgumentAliases)
    "{{PUBLIC_MEMBER_FIELDS}}" = ConvertTo-RegexAlternation $PublicMemberFields
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

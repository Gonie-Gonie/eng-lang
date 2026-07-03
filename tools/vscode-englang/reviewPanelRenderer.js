const fs = require("fs");
const path = require("path");
const {
  LAST_RUN_ARTIFACTS,
  lastRunArtifactDisplay
} = require("./artifactRegistry");
const {
  moduleStatusDisplay,
  moduleStatusDetailDisplay,
  moduleBackingLabel
} = require("./moduleStatus");

function renderReviewSummaryHtml(review, sourcePath, nonce, artifactLinks = []) {
  const doc = normalizedReviewDocument(review);
  const contract = doc.root_contract || doc.rootContract || {};
  const diagnostics = firstReviewArray(doc, review, "diagnostics");
  const inputs = reviewArray(doc, "inputs");
  const calculations = reviewArray(doc, "calculations");
  const symbols = reviewArray(doc, "symbols");
  const units = reviewArray(doc, "units_quantities", "unitsQuantities");
  const schemas = reviewArray(doc, "schemas");
  const timeAxes = reviewArray(doc, "time_axes", "timeAxes");
  const derivedValues = reviewArray(doc, "derived_values", "derivedValues");
  const tableTransforms = reviewArray(doc, "table_transforms", "tableTransforms");
  const outputs = reviewArray(doc, "report_outputs", "reportOutputs");
  const validations = reviewArray(doc, "validations");
  const sideEffects = reviewArray(doc, "side_effects", "sideEffects");
  const boundaries = reviewArray(doc, "external_boundaries", "externalBoundaries");
  const fallbacks = reviewArray(doc, "fallbacks");
  const risks = reviewArray(doc, "risks");
  const caches = reviewArray(doc, "caches");
  const modules = reviewArray(doc, "workflow_modules", "workflowModules");
  const sectionHashes = doc.section_hashes || doc.sectionHashes || {};
  const nativeModuleCount = modules.filter((module) => moduleStatusCategory(module) === "native").length;
  const plannedModuleCount = modules.filter((module) => moduleStatusCategory(module) === "planned").length;
  const internalModuleCount = modules.filter((module) => moduleStatusCategory(module) === "internal").length;

  const counts = [
    ["Inputs", countOrContract(inputs, contract, "input_count", "inputCount")],
    ["Symbols", countOrContract(symbols, contract, "symbol_count", "symbolCount")],
    ["Units", countOrContract(units, contract, "unit_quantity_count", "unitQuantityCount")],
    ["Schemas", countOrContract(schemas, contract, "schema_count", "schemaCount")],
    ["Time axes", countOrContract(timeAxes, contract, "time_axis_count", "timeAxisCount")],
    ["Derived values", derivedValues.length],
    ["Calculations", calculations.length],
    ["Caches", caches.length],
    ["Artifacts", artifactLinks.filter((artifact) => artifact.exists).length],
    ["Table transforms", tableTransforms.length],
    ["Outputs", countOrContract(outputs, contract, "report_output_count", "reportOutputCount")],
    ["Validations", countOrContract(validations, contract, "validation_count", "validationCount")],
    ["Side effects", countOrContract(sideEffects, contract, "side_effect_count", "sideEffectCount")],
    ["External boundaries", boundaries.length],
    ["Fallbacks", fallbacks.length],
    ["Risks", risks.length],
    ["Workflow modules", modules.length],
    ["Section hashes", Object.keys(sectionHashes).length]
  ];

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'nonce-${escapeAttr(nonce)}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>EngLang Review</title>
  <style>
    :root {
      color-scheme: light dark;
    }
    body {
      margin: 0;
      padding: 0;
      color: var(--vscode-editor-foreground);
      background: var(--vscode-editor-background);
      font-family: var(--vscode-font-family);
      font-size: var(--vscode-font-size);
      line-height: 1.45;
    }
    header {
      padding: 18px 22px 14px;
      border-bottom: 1px solid var(--vscode-panel-border);
      background: var(--vscode-sideBar-background);
    }
    main {
      padding: 18px 22px 28px;
    }
    h1, h2 {
      margin: 0;
      font-weight: 600;
      letter-spacing: 0;
    }
    h1 {
      font-size: 20px;
    }
    h2 {
      margin-top: 22px;
      margin-bottom: 8px;
      font-size: 14px;
    }
    code {
      color: var(--vscode-textPreformat-foreground);
      font-family: var(--vscode-editor-font-family);
      font-size: 0.95em;
      white-space: pre-wrap;
      word-break: break-word;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      table-layout: fixed;
    }
    th, td {
      padding: 7px 8px;
      border-bottom: 1px solid var(--vscode-panel-border);
      text-align: left;
      vertical-align: top;
      word-break: break-word;
    }
    th {
      color: var(--vscode-descriptionForeground);
      background: var(--vscode-editorGroupHeader-tabsBackground);
      font-size: 12px;
      font-weight: 600;
    }
    .path {
      margin-top: 4px;
      color: var(--vscode-descriptionForeground);
      word-break: break-all;
    }
    .badges {
      display: flex;
      flex-wrap: wrap;
      gap: 6px;
      margin-top: 12px;
    }
    .badge, .pill {
      display: inline-flex;
      align-items: center;
      min-height: 20px;
      padding: 1px 7px;
      border: 1px solid var(--vscode-panel-border);
      border-radius: 4px;
      background: var(--vscode-button-secondaryBackground);
      color: var(--vscode-button-secondaryForeground);
      font-size: 12px;
      white-space: nowrap;
    }
    .pill.good {
      border-color: var(--vscode-testing-iconPassed);
      color: var(--vscode-testing-iconPassed);
      background: transparent;
    }
    .pill.warn {
      border-color: var(--vscode-editorWarning-foreground);
      color: var(--vscode-editorWarning-foreground);
      background: transparent;
    }
    .pill.bad {
      border-color: var(--vscode-editorError-foreground);
      color: var(--vscode-editorError-foreground);
      background: transparent;
    }
    .grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(130px, 1fr));
      gap: 8px;
      margin-bottom: 8px;
    }
    .metric {
      padding: 9px 10px;
      border: 1px solid var(--vscode-panel-border);
      border-radius: 4px;
      background: var(--vscode-editorWidget-background);
    }
    .metric strong {
      display: block;
      margin-bottom: 2px;
      font-size: 15px;
    }
    .metric span, .muted {
      color: var(--vscode-descriptionForeground);
      font-size: 12px;
    }
    .table-wrap {
      overflow-x: auto;
      border: 1px solid var(--vscode-panel-border);
      border-radius: 4px;
    }
    .section-note {
      margin: -3px 0 8px;
      color: var(--vscode-descriptionForeground);
      font-size: 12px;
    }
    .line-button {
      display: inline-flex;
      align-items: center;
      min-height: 20px;
      padding: 0 6px;
      border: 1px solid var(--vscode-button-border, transparent);
      border-radius: 4px;
      color: var(--vscode-textLink-foreground);
      background: transparent;
      font: inherit;
      cursor: pointer;
    }
    .line-button:hover,
    .line-button:focus {
      color: var(--vscode-textLink-activeForeground);
      background: var(--vscode-list-hoverBackground);
      outline: 1px solid var(--vscode-focusBorder);
      outline-offset: 1px;
    }
  </style>
</head>
<body>
  <header>
    <h1>Review</h1>
    <div class="path">${escapeHtml(sourcePath)}</div>
    <div class="badges">
      ${badge("Status", doc.status || "-")}
      ${badge("Diagnostics", diagnostics.length)}
      ${badge("Native modules", nativeModuleCount)}
      ${badge("Planned", plannedModuleCount)}
      ${badge("Internal", internalModuleCount)}
      ${badge("Format", doc.format || "-")}
    </div>
  </header>
  <main>
    <div class="grid">
      ${counts.map(([label, value]) => `<div class="metric"><strong>${escapeHtml(value)}</strong><span>${escapeHtml(label)}</span></div>`).join("")}
    </div>

    <h2>Review Fingerprint</h2>
    <div class="table-wrap">
      <table><tbody><tr><td><code>${escapeHtml(doc.semantic_hash || doc.semanticHash || "-")}</code></td></tr></tbody></table>
    </div>

    <h2>Last Run Artifacts</h2>
    ${renderReviewTable(
      ["Artifact", "Path", "Status", "Action"],
      artifactLinks,
      "No artifact links are configured.",
      (artifact) => `<tr>
        <td><strong>${escapeHtml(artifact.label)}</strong>${artifact.detail ? `<div class="muted">${escapeHtml(artifact.detail)}</div>` : ""}</td>
        <td><code>${escapeHtml(artifact.description)}</code></td>
        <td>${statusPill(artifact.exists ? "available" : "missing")}</td>
        <td>${artifact.exists ? `<button class="line-button" type="button" data-artifact-id="${escapeAttr(artifact.id)}" title="Open ${escapeAttr(artifact.label)}">Open</button>` : `<span class="muted">Run current file first</span>`}</td>
      </tr>`
    )}

    <h2>Inputs</h2>
    ${renderReviewTable(
      ["Line", "Name", "Kind", "Type", "Default", "Required"],
      inputs,
      "No inputs.",
      (input) => `<tr>
        <td>${sourceLineCell(input)}</td>
        <td><strong>${escapeHtml(reviewValue(input, "name"))}</strong></td>
        <td>${escapeHtml(reviewValue(input, "kind"))}</td>
        <td>${escapeHtml(reviewValue(input, "type"))}</td>
        <td><code>${escapeHtml(compactText(reviewValue(input, "default"), 110))}</code></td>
        <td>${escapeHtml(String(input.required ?? false))}${input.redacted ? `<div class="muted">redacted</div>` : ""}</td>
      </tr>`
    )}

    <h2>Symbols</h2>
    ${renderReviewTable(
      ["Line", "Name", "Quantity", "Unit", "Source"],
      symbols,
      "No symbols.",
      (symbol) => `<tr>
        <td>${sourceLineCell(symbol)}</td>
        <td><strong>${escapeHtml(reviewValue(symbol, "name"))}</strong></td>
        <td>${escapeHtml(reviewValue(symbol, "quantity_kind", "quantityKind"))}</td>
        <td>${escapeHtml(reviewValue(symbol, "display_unit", "displayUnit"))}</td>
        <td>${escapeHtml(reviewValue(symbol, "source"))}</td>
      </tr>`
    )}

    <h2>Schemas</h2>
    ${renderReviewTable(
      ["Line", "Schema", "Columns", "Constraints", "Missing Policy"],
      schemas,
      "No schemas.",
      (schema) => `<tr>
        <td>${sourceLineCell(schema)}</td>
        <td><strong>${escapeHtml(reviewValue(schema, "name"))}</strong></td>
        <td>${escapeHtml(columnSummary(reviewArray(schema, "columns"), 170))}</td>
        <td>${escapeHtml(schemaRuleSummary(reviewArray(schema, "constraints"), "text", 130))}</td>
        <td>${escapeHtml(schemaRuleSummary(reviewArray(schema, "missing_policies", "missingPolicies"), "policy", 130))}</td>
      </tr>`
    )}

    <h2>Units And Quantities</h2>
    ${renderReviewTable(
      ["Line", "Name", "Quantity", "Source", "Display", "Derivation"],
      units,
      "No unit or quantity records.",
      (unit) => `<tr>
        <td>${sourceLineCell(unit)}</td>
        <td><strong>${escapeHtml(reviewValue(unit, "name"))}</strong><div class="muted">${escapeHtml(reviewValue(unit, "status"))}</div></td>
        <td>${escapeHtml(reviewValue(unit, "quantity_kind", "quantityKind"))}</td>
        <td>${escapeHtml(reviewValue(unit, "source_unit", "sourceUnit"))}</td>
        <td>${escapeHtml(reviewValue(unit, "display_unit", "displayUnit"))}<div class="muted">${escapeHtml(reviewValue(unit, "canonical_unit", "canonicalUnit"))}</div></td>
        <td>${escapeHtml(reviewList(reviewArray(unit, "derivation_steps", "derivationSteps"), 140))}</td>
      </tr>`
    )}

    <h2>Time Axes</h2>
    ${renderReviewTable(
      ["Line", "Axis", "Binding", "Role", "Source"],
      timeAxes,
      "No time axes.",
      (axis) => `<tr>
        <td>${sourceLineCell(axis)}</td>
        <td><strong>${escapeHtml(reviewValue(axis, "axis"))}</strong></td>
        <td>${escapeHtml(reviewValue(axis, "binding"))}</td>
        <td>${escapeHtml(reviewValue(axis, "role"))}</td>
        <td>${escapeHtml(reviewValue(axis, "source"))}</td>
      </tr>`
    )}

    <h2>Derived Values</h2>
    ${renderReviewTable(
      ["Line", "Name", "Expression", "Quantity", "Unit"],
      derivedValues,
      "No derived values.",
      (derived) => `<tr>
        <td>${sourceLineCell(derived)}</td>
        <td><strong>${escapeHtml(reviewValue(derived, "name"))}</strong></td>
        <td><code>${escapeHtml(compactText(reviewValue(derived, "expression"), 150))}</code></td>
        <td>${escapeHtml(reviewValue(derived, "quantity_kind", "quantityKind"))}</td>
        <td>${escapeHtml(reviewValue(derived, "display_unit", "displayUnit"))}</td>
      </tr>`
    )}

    <h2>Caches</h2>
    ${renderReviewTable(
      ["Line", "Owner", "Status", "Key", "Hash"],
      caches,
      "No cache records.",
      (cache) => `<tr>
        <td>${sourceLineCell(cache)}</td>
        <td><strong>${escapeHtml(reviewValue(cache, "owner_name", "ownerName"))}</strong><div class="muted">${escapeHtml(reviewValue(cache, "owner_kind", "ownerKind"))}</div></td>
        <td>${statusPill(reviewValue(cache, "status"))}<div class="muted">${escapeHtml(reviewValue(cache, "policy"))}</div></td>
        <td><code>${escapeHtml(compactText(reviewValue(cache, "cache_key", "cacheKey"), 130))}</code></td>
        <td><code>${escapeHtml(compactText(reviewValue(cache, "observed_hash", "observedHash"), 80))}</code></td>
      </tr>`
    )}

    <h2>Diagnostics</h2>
    ${renderReviewTable(
      ["Line", "Severity", "Code", "Message"],
      diagnostics,
      "No diagnostics.",
      (diagnostic) => `<tr>
        <td>${sourceLineCell(diagnostic)}</td>
        <td>${statusPill(severityName(diagnostic.severity))}</td>
        <td><code>${escapeHtml(reviewValue(diagnostic, "code"))}</code></td>
        <td>${escapeHtml(compactText(reviewValue(diagnostic, "message"), 180))}${diagnostic.help ? `<div class="muted">${escapeHtml(compactText(diagnostic.help, 180))}</div>` : ""}</td>
      </tr>`
    )}

    <h2>External Boundaries</h2>
    ${renderReviewTable(
      ["Line", "Name", "Target", "Status", "Risk", "Effects"],
      boundaries,
      "No external boundaries.",
      (boundary) => `<tr>
        <td>${sourceLineCell(boundary)}</td>
        <td><strong>${escapeHtml(reviewValue(boundary, "name", "kind"))}</strong><div class="muted">${escapeHtml(reviewValue(boundary, "kind"))}</div></td>
        <td><code>${escapeHtml(compactText(reviewValue(boundary, "target"), 120))}</code></td>
        <td>${statusPill(reviewValue(boundary, "status"))}<div class="muted">${escapeHtml(boundary.status_class || boundary.statusClass || "")} ${escapeHtml(boundary.status_code ?? boundary.statusCode ?? "")}</div></td>
        <td>${statusPill(reviewValue(boundary, "risk_level", "riskLevel"))}</td>
        <td>${escapeHtml(reviewList(reviewArray(boundary, "side_effects", "sideEffects"), 120))}</td>
      </tr>`
    )}

    <h2>Side Effects</h2>
    ${renderReviewTable(
      ["Line", "Kind", "Target", "Status", "Risk"],
      sideEffects,
      "No side effects.",
      (effect) => `<tr>
        <td>${sourceLineCell(effect)}</td>
        <td><strong>${escapeHtml(reviewValue(effect, "kind"))}</strong></td>
        <td><code>${escapeHtml(compactText(reviewValue(effect, "target", "path"), 120))}</code></td>
        <td>${statusPill(reviewValue(effect, "status"))}</td>
        <td>${statusPill(reviewValue(effect, "risk_level", "riskLevel"))}</td>
      </tr>`
    )}

    <h2>Table Transforms</h2>
    ${renderReviewTable(
      ["Line", "Binding", "Operation", "Source", "Predicates", "Status"],
      tableTransforms,
      "No table transforms.",
      (transform) => `<tr>
        <td>${sourceLineCell(transform)}</td>
        <td><strong>${escapeHtml(reviewValue(transform, "binding"))}</strong><div class="muted">${escapeHtml(reviewValue(transform, "schema_name", "schemaName"))}</div></td>
        <td>${escapeHtml(reviewValue(transform, "operation"))}</td>
        <td>${escapeHtml(reviewValue(transform, "source_table", "sourceTable"))}</td>
        <td>${escapeHtml(predicateSummary(reviewArray(transform, "predicates"), 160))}</td>
        <td>${statusPill(reviewValue(transform, "status"))}</td>
      </tr>`
    )}

    <h2>Calculations</h2>
    ${renderReviewTable(
      ["Line", "Name", "Expression", "Inputs", "Output"],
      calculations,
      "No calculations.",
      (calculation) => `<tr>
        <td>${sourceLineCell(calculation)}</td>
        <td><strong>${escapeHtml(reviewValue(calculation, "name"))}</strong><div class="muted">${escapeHtml(reviewValue(calculation, "kind"))}</div></td>
        <td><code>${escapeHtml(compactText(reviewValue(calculation, "expression"), 130))}</code></td>
        <td>${escapeHtml(reviewList(reviewArray(calculation, "input_symbols", "inputSymbols"), 100))}</td>
        <td>${escapeHtml(reviewValue(calculation, "output_quantity", "outputQuantity"))}</td>
      </tr>`
    )}

    <h2>Report Outputs</h2>
    ${renderReviewTable(
      ["Line", "Kind", "Source", "Quantity", "Status"],
      outputs,
      "No report outputs.",
      (outputItem) => `<tr>
        <td>${sourceLineCell(outputItem)}</td>
        <td><strong>${escapeHtml(reviewValue(outputItem, "kind"))}</strong></td>
        <td>${escapeHtml(reviewValue(outputItem, "source"))}</td>
        <td>${escapeHtml(reviewValue(outputItem, "quantity_kind", "quantityKind"))}</td>
        <td>${statusPill(reviewValue(outputItem, "status"))}</td>
      </tr>`
    )}

    <h2>Validations</h2>
    ${renderReviewTable(
      ["Line", "Target", "Kind", "Status", "Reason"],
      validations,
      "No validations.",
      (validation) => `<tr>
        <td>${sourceLineCell(validation)}</td>
        <td><strong>${escapeHtml(reviewValue(validation, "target", "name"))}</strong></td>
        <td>${escapeHtml(reviewValue(validation, "kind", "category"))}</td>
        <td>${statusPill(reviewValue(validation, "status"))}</td>
        <td>${escapeHtml(compactText(reviewValue(validation, "reason", "summary"), 140))}</td>
      </tr>`
    )}

    <h2>Fallbacks</h2>
    ${renderReviewTable(
      ["Line", "Kind", "Target", "Method", "Risk", "Assumption"],
      fallbacks,
      "No fallbacks.",
      (fallback) => `<tr>
        <td>${sourceLineCell(fallback)}</td>
        <td><strong>${escapeHtml(reviewValue(fallback, "kind"))}</strong></td>
        <td>${escapeHtml(reviewValue(fallback, "target"))}</td>
        <td>${escapeHtml(reviewValue(fallback, "method"))}</td>
        <td>${statusPill(reviewValue(fallback, "risk_level", "riskLevel"))}</td>
        <td>${escapeHtml(compactText(reviewValue(fallback, "assumption", "reason"), 140))}</td>
      </tr>`
    )}

    <h2>Risks</h2>
    ${renderReviewTable(
      ["Line", "Category", "Level", "Severity", "Summary"],
      risks,
      "No review risks.",
      (risk) => `<tr>
        <td>${sourceLineCell(risk)}</td>
        <td><strong>${escapeHtml(reviewValue(risk, "category"))}</strong></td>
        <td>${statusPill(reviewValue(risk, "level"))}</td>
        <td>${escapeHtml(reviewValue(risk, "severity"))}</td>
        <td>${escapeHtml(compactText(reviewValue(risk, "summary"), 150))}</td>
      </tr>`
    )}

    <h2>Workflow Modules</h2>
    <div class="section-note">Native means compiler/runtime-backed for the current public surface.</div>
    ${renderReviewTable(
      ["Module", "Status", "Backing", "Purpose", "Artifacts", "Tests"],
      modules,
      "No workflow modules.",
      (module) => `<tr>
        <td><strong>${escapeHtml(reviewValue(module, "name"))}</strong></td>
        <td>${statusPill(moduleStatusDisplay(module))}<div class="muted">${escapeHtml(compactText(moduleStatusDetailDisplay(module), 100))}</div></td>
        <td>${escapeHtml(moduleBackingLabel(module))}</td>
        <td>${escapeHtml(compactText(reviewValue(module, "purpose"), 160))}</td>
        <td>${escapeHtml(module.artifact_count ?? module.artifactCount ?? reviewArray(module, "artifacts").length)}</td>
        <td>${escapeHtml(module.test_count ?? module.testCount ?? reviewArray(module, "tests").length)}</td>
      </tr>`
    )}
  </main>
  <script nonce="${escapeAttr(nonce)}">
    (() => {
      const vscode = acquireVsCodeApi();
      document.addEventListener("click", (event) => {
        const artifactTarget = event.target.closest("[data-artifact-id]");
        if (artifactTarget) {
          vscode.postMessage({
            type: "openArtifact",
            artifactId: artifactTarget.getAttribute("data-artifact-id")
          });
          return;
        }
        const target = event.target.closest("[data-source-line]");
        if (!target) {
          return;
        }
        const line = Number(target.getAttribute("data-source-line"));
        if (Number.isFinite(line) && line > 0) {
          vscode.postMessage({ type: "openSourceLine", line });
        }
      });
    })();
  </script>
</body>
</html>`;
}

function reviewPanelArtifacts(root, artifacts = LAST_RUN_ARTIFACTS) {
  return artifacts.map((artifact) => {
    const artifactPath = root ? path.join(root, ...artifact.relativePath) : undefined;
    const display = lastRunArtifactDisplay(artifact, root);
    return {
      id: artifact.id,
      label: display.label,
      detail: display.detail,
      description: artifact.description,
      exists: Boolean(artifactPath && fs.existsSync(artifactPath))
    };
  });
}

function normalizedReviewDocument(review) {
  if (review && typeof review === "object") {
    return review.review_document || review.reviewDocument || review;
  }
  return {};
}

function firstReviewArray(primary, fallback, snakeKey, camelKey = snakeKey) {
  const primaryValue = reviewArray(primary, snakeKey, camelKey);
  if (primaryValue.length > 0) {
    return primaryValue;
  }
  return reviewArray(fallback, snakeKey, camelKey);
}

function reviewArray(object, snakeKey, camelKey = snakeKey) {
  const value = object?.[snakeKey] ?? object?.[camelKey];
  return Array.isArray(value) ? value : [];
}

function reviewValue(object, snakeKey, camelKey = snakeKey, fallback = "-") {
  if (!object || typeof object !== "object") {
    return fallback;
  }
  const value = object[snakeKey] ?? object[camelKey];
  return value === null || value === undefined || value === "" ? fallback : value;
}

function countOrContract(items, contract, snakeKey, camelKey) {
  if (items.length > 0) {
    return items.length;
  }
  return contract?.[snakeKey] ?? contract?.[camelKey] ?? 0;
}

function lineValue(item) {
  return item?.source_span?.line
    ?? item?.sourceSpan?.line
    ?? item?.source_line
    ?? item?.sourceLine
    ?? item?.line
    ?? "-";
}

function sourceLineCell(item) {
  const line = lineValue(item);
  const lineNumber = Number(line);
  if (!Number.isFinite(lineNumber) || lineNumber < 1) {
    return escapeHtml(line);
  }
  const safeLine = Math.trunc(lineNumber);
  return `<button class="line-button" type="button" data-source-line="${escapeAttr(safeLine)}" title="Open source line ${escapeAttr(safeLine)}">L${escapeHtml(safeLine)}</button>`;
}

function reviewList(value, limit = 120) {
  if (!Array.isArray(value) || value.length === 0) {
    return "-";
  }
  return compactText(
    value.map((item) => {
      if (item && typeof item === "object") {
        return JSON.stringify(item);
      }
      return String(item);
    }).join("; "),
    limit
  );
}

function columnSummary(columns, limit = 140) {
  if (!Array.isArray(columns) || columns.length === 0) {
    return "-";
  }
  return compactText(
    columns.map((column) => {
      const name = column.name || "-";
      const type = column.type || "-";
      const unit = column.unit ? ` [${column.unit}]` : "";
      const flags = [
        column.is_index || column.isIndex ? "index" : "",
        column.optional ? "optional" : ""
      ].filter(Boolean);
      return `${name}: ${type}${unit}${flags.length ? ` (${flags.join(", ")})` : ""}`;
    }).join("; "),
    limit
  );
}

function schemaRuleSummary(items, valueKey, limit = 120) {
  if (!Array.isArray(items) || items.length === 0) {
    return "-";
  }
  return compactText(
    items.map((item) => {
      if (!item || typeof item !== "object") {
        return String(item);
      }
      const column = item.column ? `${item.column}: ` : "";
      return `${column}${item[valueKey] || item.text || item.policy || JSON.stringify(item)}`;
    }).join("; "),
    limit
  );
}

function predicateSummary(predicates, limit = 140) {
  if (!Array.isArray(predicates) || predicates.length === 0) {
    return "-";
  }
  const text = predicates.map((predicate) => {
    const expression = predicate.expression || [
      predicate.column,
      predicate.operator,
      predicate.value
    ].filter((part) => part !== null && part !== undefined && part !== "").join(" ");
    return `${expression || "-"} (${predicate.status || "-"})`;
  }).join("; ");
  return compactText(text, limit);
}

function compactText(value, limit = 120) {
  if (value === null || value === undefined || value === "") {
    return "-";
  }
  const text = typeof value === "string" ? value : String(value);
  if (text.length <= limit) {
    return text;
  }
  return `${text.slice(0, Math.max(0, limit - 3))}...`;
}

function renderReviewTable(headers, rows, emptyLabel, renderRow) {
  const headerHtml = headers.map((header) => `<th>${escapeHtml(header)}</th>`).join("");
  const bodyHtml = rows.length > 0
    ? rows.map(renderRow).join("")
    : `<tr><td colspan="${headers.length}" class="muted">${escapeHtml(emptyLabel)}</td></tr>`;
  return `<div class="table-wrap"><table><thead><tr>${headerHtml}</tr></thead><tbody>${bodyHtml}</tbody></table></div>`;
}

function badge(label, value) {
  return `<span class="badge">${escapeHtml(label)} ${escapeHtml(value)}</span>`;
}

function statusPill(value) {
  return `<span class="pill ${statusClass(value)}">${escapeHtml(value)}</span>`;
}

function statusClass(value) {
  const text = String(value ?? "").toLowerCase();
  if (!text || text === "-") {
    return "";
  }
  if (
    text.includes("error") ||
    text.includes("fail") ||
    text.includes("high") ||
    text.includes("blocked") ||
    text.includes("invalid")
  ) {
    return "bad";
  }
  if (
    text.includes("warn") ||
    text.includes("medium") ||
    text.includes("stale") ||
    text.includes("missing") ||
    text.includes("planned")
  ) {
    return "warn";
  }
  if (
    text.includes("success") ||
    text.includes("supported") ||
    text.includes("native") ||
    text.includes("accepted") ||
    text.includes("declared") ||
    text.includes("fixture") ||
    text.includes("passed") ||
    text.includes("ok")
  ) {
    return "good";
  }
  return "";
}

function moduleStatusCategory(module) {
  const status = String(module?.status || "").toLowerCase();
  if (status.startsWith("supported") || status.includes("native")) {
    return "native";
  }
  if (status.includes("internal")) {
    return "internal";
  }
  if (status.includes("planned")) {
    return "planned";
  }
  return "other";
}

function severityName(severity) {
  if (severity === 1 || severity === "error") {
    return "error";
  }
  if (severity === 2 || severity === "warning") {
    return "warning";
  }
  return "info";
}

function escapeHtml(value) {
  return String(value ?? "")
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#39;");
}

function escapeAttr(value) {
  return escapeHtml(value);
}

module.exports = {
  firstReviewArray,
  lineValue,
  normalizedReviewDocument,
  renderReviewSummaryHtml,
  reviewArray,
  reviewPanelArtifacts,
  reviewValue
};

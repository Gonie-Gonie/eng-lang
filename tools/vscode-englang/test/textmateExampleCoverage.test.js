"use strict";

const fs = require("fs");
const path = require("path");

const extensionRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(extensionRoot, "..", "..");

function existingDirectory(candidate) {
  try {
    return fs.statSync(candidate).isDirectory();
  } catch (_error) {
    return false;
  }
}

function addCandidate(candidates, nodeModulesRoot, onigurumaWasmPath) {
  if (!nodeModulesRoot) {
    return;
  }
  const resolved = path.resolve(nodeModulesRoot);
  const textmateRoot = path.join(resolved, "vscode-textmate");
  const onigurumaRoot = path.join(resolved, "vscode-oniguruma");
  const wasmPath = onigurumaWasmPath
    ? path.resolve(onigurumaWasmPath)
    : path.join(onigurumaRoot, "release", "onig.wasm");
  if (
    !existingDirectory(textmateRoot) ||
    !existingDirectory(onigurumaRoot) ||
    !fs.existsSync(wasmPath)
  ) {
    return;
  }
  if (
    !candidates.some(
      (candidate) =>
        candidate.textmateRoot === textmateRoot &&
        candidate.onigurumaRoot === onigurumaRoot &&
        candidate.wasmPath === wasmPath
    )
  ) {
    candidates.push({ nodeModulesRoot: resolved, textmateRoot, onigurumaRoot, wasmPath });
  }
}

function addVscodeAppCandidates(candidates, appRoot) {
  if (!appRoot) {
    return;
  }
  addCandidate(candidates, path.join(appRoot, "node_modules"));
  addCandidate(
    candidates,
    path.join(appRoot, "node_modules.asar"),
    path.join(
      appRoot,
      "node_modules.asar.unpacked",
      "vscode-oniguruma",
      "release",
      "onig.wasm"
    )
  );
}

function addVscodeInstallCandidates(candidates, installRoot) {
  if (!installRoot || !existingDirectory(installRoot)) {
    return;
  }
  addVscodeAppCandidates(candidates, path.join(installRoot, "resources", "app"));
  for (const entry of fs.readdirSync(installRoot, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      addVscodeAppCandidates(
        candidates,
        path.join(installRoot, entry.name, "resources", "app")
      );
    }
  }
}

function vscodeNodeModuleCandidates() {
  const candidates = [];
  addCandidate(candidates, process.env.VSCODE_NODE_MODULES);
  addCandidate(candidates, path.join(extensionRoot, "node_modules"));
  addCandidate(candidates, path.join(repoRoot, "node_modules"));

  const executableRoot = path.dirname(process.execPath);
  addVscodeInstallCandidates(candidates, executableRoot);
  addVscodeInstallCandidates(candidates, path.dirname(executableRoot));

  if (process.platform === "win32") {
    addVscodeInstallCandidates(
      candidates,
      process.env.LOCALAPPDATA &&
        path.join(process.env.LOCALAPPDATA, "Programs", "Microsoft VS Code")
    );
    addVscodeInstallCandidates(
      candidates,
      process.env.ProgramFiles && path.join(process.env.ProgramFiles, "Microsoft VS Code")
    );
    addVscodeInstallCandidates(
      candidates,
      process.env["ProgramFiles(x86)"] &&
        path.join(process.env["ProgramFiles(x86)"], "Microsoft VS Code")
    );
  } else if (process.platform === "darwin") {
    addCandidate(
      candidates,
      "/Applications/Visual Studio Code.app/Contents/Resources/app/node_modules"
    );
  } else {
    addCandidate(candidates, "/usr/share/code/resources/app/node_modules");
    addCandidate(candidates, "/usr/lib/code/resources/app/node_modules");
    addCandidate(candidates, "/opt/visual-studio-code/resources/app/node_modules");
  }
  return candidates;
}

function loadTextMateRuntime() {
  for (const candidate of vscodeNodeModuleCandidates()) {
    return {
      nodeModulesRoot: candidate.nodeModulesRoot,
      oniguruma: require(candidate.onigurumaRoot),
      textmate: require(candidate.textmateRoot),
      wasmPath: candidate.wasmPath
    };
  }
  return undefined;
}

function readJson(relativePath) {
  return JSON.parse(fs.readFileSync(path.join(extensionRoot, relativePath), "utf8"));
}

function foregroundScopeSelectors(theme) {
  const selectors = [];
  for (const rule of theme.tokenColors || []) {
    if (!rule.settings || !rule.settings.foreground) {
      continue;
    }
    const scopes = Array.isArray(rule.scope) ? rule.scope : [rule.scope];
    for (const scopeList of scopes) {
      for (const selector of String(scopeList || "").split(",")) {
        const trimmed = selector.trim();
        if (trimmed && !selectors.includes(trimmed)) {
          selectors.push(trimmed);
        }
      }
    }
  }
  return selectors;
}

function scopeHasForeground(scopes, selectors) {
  return selectors.some((selector) =>
    scopes.some((scope) => scope === selector || scope.startsWith(`${selector}.`))
  );
}

function collectEngFiles(root) {
  const files = [];
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    const entryPath = path.join(root, entry.name);
    if (entry.isDirectory()) {
      files.push(...collectEngFiles(entryPath));
    } else if (entry.isFile() && entry.name.endsWith(".eng")) {
      files.push(entryPath);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function semanticSnapshotBundle() {
  const bundlePath = process.env.ENGLANG_TEXTMATE_SEMANTIC_SNAPSHOTS;
  if (!bundlePath || !fs.existsSync(bundlePath)) {
    return new Map();
  }
  const bundle = JSON.parse(fs.readFileSync(bundlePath, "utf8").replace(/^\uFEFF/, ""));
  if (bundle.format !== "englang-textmate-semantic-snapshots-v1") {
    throw new Error("unexpected TextMate semantic snapshot bundle format");
  }
  const snapshots = new Map();
  for (const snapshot of bundle.snapshots || []) {
    const sourcePath = String(snapshot.path || "").replace(/\\/g, "/");
    if (!sourcePath || !Array.isArray(snapshot.tokens)) {
      throw new Error("invalid TextMate semantic snapshot bundle entry");
    }
    snapshots.set(sourcePath, snapshot.tokens);
  }
  return snapshots;
}

function semanticFallbackScopes(token, scopeMap) {
  const modifierScopes = (token.modifiers || []).flatMap((modifier) => {
    const scopes = scopeMap[token.type + "." + modifier];
    return Array.isArray(scopes) ? scopes : [];
  });
  const typeScopes = scopeMap[token.type];
  const selectedScopes =
    modifierScopes.length > 0
      ? modifierScopes
      : Array.isArray(typeScopes)
        ? typeScopes
        : [];
  return Array.from(
    new Set(selectedScopes)
  );
}

function scopesIntersect(actualScopes, expectedScopes) {
  return expectedScopes.some((expected) =>
    actualScopes.some(
      (actual) => actual === expected || actual.startsWith(expected + ".")
    )
  );
}

function needsSemanticTextMateParity(token) {
  if (token.type === "keyword" || token.type === "modifier") {
    return true;
  }
  if (token.type !== "function" && token.type !== "method") {
    return false;
  }
  const roleModifiers = new Set([
    "defaultLibrary",
    "deprecated",
    "timeseries",
    "uncertain",
    "sideEffect",
    "external",
    "validation",
    "report",
    "solver",
    "model",
    "db",
    "cache",
    "workflowStep",
    "path",
    "temporal"
  ]);
  return (token.modifiers || []).some((modifier) => roleModifiers.has(modifier));
}

function escapeRegex(text) {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function labelPattern(labels) {
  const alternatives = labels
    .slice()
    .sort((left, right) => right.length - left.length || left.localeCompare(right))
    .map(escapeRegex)
    .join("|");
  return new RegExp(`(^|[^A-Za-z0-9_])(${alternatives})(?![A-Za-z0-9_])`, "g");
}

function overlappingScopes(tokens, startIndex, endIndex) {
  const scopes = [];
  for (const token of tokens) {
    if (token.endIndex <= startIndex || token.startIndex >= endIndex) {
      continue;
    }
    for (const scope of token.scopes) {
      if (!scopes.includes(scope)) {
        scopes.push(scope);
      }
    }
  }
  return scopes;
}

function isCommentOrString(scopes) {
  return scopes.some((scope) => scope.startsWith("comment.") || scope.startsWith("string."));
}

function displayPath(filePath) {
  return path.relative(repoRoot, filePath).split(path.sep).join("/");
}

async function main() {
  const runtime = loadTextMateRuntime();
  if (!runtime) {
    const message =
      "VS Code TextMate example coverage skipped: vscode-textmate/vscode-oniguruma were not found. " +
      "Install VS Code or set VSCODE_NODE_MODULES to its resources/app/node_modules directory.";
    if (
      process.env.ENGLANG_REQUIRE_TEXTMATE_RUNTIME === "1" ||
      Boolean(process.versions?.electron)
    ) {
      throw new Error(message);
    }
    console.log(message);
    return;
  }

  const wasmBuffer = fs.readFileSync(runtime.wasmPath);
  const wasm = wasmBuffer.buffer.slice(
    wasmBuffer.byteOffset,
    wasmBuffer.byteOffset + wasmBuffer.byteLength
  );
  await runtime.oniguruma.loadWASM(wasm);

  const rawGrammar = readJson(path.join("syntaxes", "eng.tmLanguage.json"));
  const registry = new runtime.textmate.Registry({
    onigLib: Promise.resolve({
      createOnigScanner(patterns) {
        return new runtime.oniguruma.OnigScanner(patterns);
      },
      createOnigString(text) {
        return new runtime.oniguruma.OnigString(text);
      }
    }),
    loadGrammar: async (scopeName) => (scopeName === "source.englang" ? rawGrammar : null)
  });
  const grammar = await registry.loadGrammar("source.englang");
  if (!grammar) {
    throw new Error("VS Code TextMate runtime could not load source.englang");
  }

  const metadata = readJson(
    path.join("generated", "editor", "englang-editor-metadata.json")
  );
  const extensionPackage = readJson("package.json");
  const semanticScopeRule = (extensionPackage.contributes?.semanticTokenScopes || [])
    .find((rule) => rule.language === "englang");
  if (!semanticScopeRule?.scopes) {
    throw new Error("VS Code package is missing EngLang semanticTokenScopes");
  }
  const semanticSnapshots = semanticSnapshotBundle();
  const keywords = Array.from(new Set(metadata.syntax_catalog?.keywords || []));
  if (keywords.length === 0) {
    throw new Error("editor metadata syntax_catalog.keywords is empty");
  }
  const themes = [
    {
      label: "EngLang Dark",
      selectors: foregroundScopeSelectors(
        readJson(path.join("themes", "englang-dark-color-theme.json"))
      )
    },
    {
      label: "EngLang Light",
      selectors: foregroundScopeSelectors(
        readJson(path.join("themes", "englang-light-color-theme.json"))
      )
    }
  ];
  for (const theme of themes) {
    if (theme.selectors.length === 0) {
      throw new Error(`${theme.label} has no foreground token scopes`);
    }
  }

  const examplesRoot = path.join(repoRoot, "examples");
  const files = collectEngFiles(examplesRoot);
  if (files.length === 0) {
    throw new Error("no examples/**/*.eng files were found");
  }

  const keywordRegex = labelPattern(keywords);
  const requiredScopes = [
    {
      count: 0,
      description: "file helper call",
      regex: /(^|[^A-Za-z0-9_])(file)(?=\s*\()/g,
      scope: "support.function.external-boundary.englang"
    },
    {
      count: 0,
      description: "missing-policy option",
      regex: /(^|[^A-Za-z0-9_])(max_gap)(?=\s*=)/g,
      scope: "variable.parameter.property.englang"
    },
    {
      count: 0,
      description: "compound derivative unit",
      regex: /(^|[^A-Za-z0-9_])(K\/s)(?![A-Za-z0-9_/])/g,
      scope: "constant.other.unit.englang"
    }
  ];
  const failures = [];
  let checkedOccurrences = 0;
  let checkedSemanticOccurrences = 0;
  let semanticSnapshotCount = 0;
  let skippedLexicalOccurrences = 0;
  let interpolateOccurrences = 0;

  for (const filePath of files) {
    const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
    const semanticTokensByLine = new Map();
    const semanticSnapshot = semanticSnapshots.get(displayPath(filePath));
    if (semanticSnapshots.size > 0 && !semanticSnapshot) {
      throw new Error("semantic snapshot bundle is missing " + displayPath(filePath));
    }
    if (semanticSnapshot) {
      semanticSnapshotCount += 1;
      for (const token of semanticSnapshot) {
        const lineTokens = semanticTokensByLine.get(token.line) || [];
        lineTokens.push(token);
        semanticTokensByLine.set(token.line, lineTokens);
      }
    }
    let ruleStack = runtime.textmate.INITIAL;
    for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
      const line = lines[lineIndex];
      const tokenized = grammar.tokenizeLine(line, ruleStack);
      ruleStack = tokenized.ruleStack;
      for (const semanticToken of semanticTokensByLine.get(lineIndex) || []) {
        if (!needsSemanticTextMateParity(semanticToken)) {
          continue;
        }
        const startIndex = Number(semanticToken.start);
        const endIndex = startIndex + Number(semanticToken.length);
        const scopes = overlappingScopes(tokenized.tokens, startIndex, endIndex);
        const expectedScopes = semanticFallbackScopes(
          semanticToken,
          semanticScopeRule.scopes
        );
        checkedSemanticOccurrences += 1;
        if (expectedScopes.length === 0) {
          failures.push({
            filePath,
            keyword: line.slice(startIndex, endIndex),
            lineIndex,
            scopes,
            reason:
              "semantic selector has no TextMate fallback mapping: " +
              [semanticToken.type, ...(semanticToken.modifiers || [])].join(".")
          });
        } else if (!scopesIntersect(scopes, expectedScopes)) {
          failures.push({
            filePath,
            keyword: line.slice(startIndex, endIndex),
            lineIndex,
            scopes,
            reason:
              "semantic/TextMate mismatch for " +
              semanticToken.type +
              ((semanticToken.modifiers || []).length > 0
                ? "." + semanticToken.modifiers.join(".")
                : "") +
              "; expected one of " +
              expectedScopes.join(",")
          });
        }
      }
      keywordRegex.lastIndex = 0;
      for (
        let match = keywordRegex.exec(line);
        match;
        match = keywordRegex.exec(line)
      ) {
        const keyword = match[2];
        const startIndex = match.index + match[1].length;
        const endIndex = startIndex + keyword.length;
        const scopes = overlappingScopes(tokenized.tokens, startIndex, endIndex);
        if (isCommentOrString(scopes)) {
          skippedLexicalOccurrences += 1;
          continue;
        }

        checkedOccurrences += 1;
        const reasons = [];
        if (keyword === "interpolate") {
          interpolateOccurrences += 1;
          if (!scopes.includes("keyword.control.validation.englang")) {
            reasons.push("missing keyword.control.validation.englang");
          }
        }

        const missingThemes = themes
          .filter((theme) => !scopeHasForeground(scopes, theme.selectors))
          .map((theme) => theme.label);
        if (missingThemes.length > 0) {
          reasons.push(`no foreground scope in ${missingThemes.join(" and ")}`);
        }
        if (reasons.length > 0) {
          failures.push({
            filePath,
            keyword,
            lineIndex,
            scopes,
            reason: reasons.join("; ")
          });
        }
      }

      for (const requirement of requiredScopes) {
        requirement.regex.lastIndex = 0;
        for (
          let match = requirement.regex.exec(line);
          match;
          match = requirement.regex.exec(line)
        ) {
          const label = match[2];
          const startIndex = match.index + match[1].length;
          const endIndex = startIndex + label.length;
          const scopes = overlappingScopes(tokenized.tokens, startIndex, endIndex);
          if (isCommentOrString(scopes)) {
            continue;
          }
          requirement.count += 1;
          if (!scopes.includes(requirement.scope)) {
            failures.push({
              filePath,
              keyword: label,
              lineIndex,
              scopes,
              reason: `${requirement.description} is missing ${requirement.scope}`
            });
          }
        }
      }
    }
  }

  if (checkedOccurrences === 0) {
    throw new Error("no syntax catalog keyword occurrences were checked");
  }
  if (semanticSnapshots.size > 0 && checkedSemanticOccurrences === 0) {
    throw new Error("semantic snapshot bundle contains no TextMate parity tokens");
  }
  if (interpolateOccurrences === 0) {
    throw new Error("examples must retain an interpolate policy highlighting fixture");
  }
  const missingRequiredFixtures = requiredScopes.filter((requirement) => requirement.count === 0);
  if (missingRequiredFixtures.length > 0) {
    throw new Error(
      `examples are missing TextMate role fixture(s): ${missingRequiredFixtures
        .map((requirement) => requirement.description)
        .join(", ")}`
    );
  }
  if (failures.length > 0) {
    const groupedFailures = new Map();
    for (const failure of failures) {
      const semanticReason = failure.reason.split("; expected one of ")[0];
      const roleScopes = failure.scopes
        .filter((scope) => scope !== "source.englang" && !scope.startsWith("meta."))
        .join(",");
      const key =
        semanticReason +
        " [" +
        failure.keyword +
        "] <= " +
        (roleScopes || "<none>");
      groupedFailures.set(key, (groupedFailures.get(key) || 0) + 1);
    }
    const groupDetails = Array.from(groupedFailures.entries())
      .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
      .slice(0, 30)
      .map(([group, count]) => count + "x " + group)
      .join("\n");
    const details = failures
      .slice(0, 20)
      .map(
        (failure) =>
          `${displayPath(failure.filePath)}:${failure.lineIndex + 1} ${failure.keyword}: ` +
          `${failure.reason}; scopes=${failure.scopes.join(",") || "<none>"}`
      )
      .join("\n");
    const remainder = failures.length > 20 ? `\n... ${failures.length - 20} more` : "";
    throw new Error(
      `VS Code TextMate example coverage found ${failures.length} gap(s):\n${groupDetails}\n\n${details}${remainder}`
    );
  }

  console.log(
    `VS Code TextMate example coverage passed. Checked ${checkedOccurrences} keyword occurrence(s) ` +
      `and ${requiredScopes.reduce((total, requirement) => total + requirement.count, 0)} ` +
      `role-sensitive occurrence(s), ${checkedSemanticOccurrences} semantic/TextMate parity ` +
      `occurrence(s) across ${files.length} example file(s) and ${semanticSnapshotCount} semantic ` +
      `snapshot(s); skipped ` +
      `${skippedLexicalOccurrences} string/comment occurrence(s).`
  );
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exitCode = 1;
});

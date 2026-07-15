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

function addCandidate(candidates, candidate) {
  if (!candidate || !existingDirectory(candidate)) {
    return;
  }
  const resolved = path.resolve(candidate);
  if (!candidates.includes(resolved)) {
    candidates.push(resolved);
  }
}

function addVscodeInstallCandidates(candidates, installRoot) {
  if (!installRoot || !existingDirectory(installRoot)) {
    return;
  }
  addCandidate(candidates, path.join(installRoot, "resources", "app", "node_modules"));
  for (const entry of fs.readdirSync(installRoot, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      addCandidate(
        candidates,
        path.join(installRoot, entry.name, "resources", "app", "node_modules")
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
  for (const nodeModulesRoot of vscodeNodeModuleCandidates()) {
    const textmateRoot = path.join(nodeModulesRoot, "vscode-textmate");
    const onigurumaRoot = path.join(nodeModulesRoot, "vscode-oniguruma");
    const wasmPath = path.join(onigurumaRoot, "release", "onig.wasm");
    if (
      !existingDirectory(textmateRoot) ||
      !existingDirectory(onigurumaRoot) ||
      !fs.existsSync(wasmPath)
    ) {
      continue;
    }
    return {
      nodeModulesRoot,
      oniguruma: require(onigurumaRoot),
      textmate: require(textmateRoot),
      wasmPath
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
    if (process.env.ENGLANG_REQUIRE_TEXTMATE_RUNTIME === "1") {
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
  let skippedLexicalOccurrences = 0;
  let interpolateOccurrences = 0;

  for (const filePath of files) {
    const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
    let ruleStack = runtime.textmate.INITIAL;
    for (let lineIndex = 0; lineIndex < lines.length; lineIndex += 1) {
      const line = lines[lineIndex];
      const tokenized = grammar.tokenizeLine(line, ruleStack);
      ruleStack = tokenized.ruleStack;
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
      `VS Code TextMate example coverage found ${failures.length} gap(s):\n${details}${remainder}`
    );
  }

  console.log(
    `VS Code TextMate example coverage passed. Checked ${checkedOccurrences} keyword occurrence(s) ` +
      `and ${requiredScopes.reduce((total, requirement) => total + requirement.count, 0)} ` +
      `role-sensitive occurrence(s) across ${files.length} example file(s); skipped ` +
      `${skippedLexicalOccurrences} string/comment occurrence(s).`
  );
}

main().catch((error) => {
  console.error(error.stack || error.message || String(error));
  process.exitCode = 1;
});

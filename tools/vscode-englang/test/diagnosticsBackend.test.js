"use strict";

const assert = require("assert");
const childProcess = require("child_process");
const Module = require("module");

class Range {
  constructor(startLine, startCharacter, endLine, endCharacter) {
    this.start = { line: startLine, character: startCharacter };
    this.end = { line: endLine, character: endCharacter };
  }
}

class Diagnostic {
  constructor(range, message, severity) {
    this.range = range;
    this.message = message;
    this.severity = severity;
  }
}

class Uri {
  constructor(value) {
    this.value = value;
  }

  static parse(value) {
    return new Uri(value);
  }

  toString() {
    return this.value;
  }
}

const vscodeMock = {
  Diagnostic,
  DiagnosticSeverity: {
    Error: 0,
    Warning: 1,
    Information: 2
  },
  DiagnosticTag: {
    Deprecated: 1,
    Unnecessary: 2
  },
  Range,
  Uri,
  window: {
    activeTextEditor: undefined,
    showInformationMessage() {},
    showWarningMessage() {}
  },
  workspace: {
    getConfiguration() {
      return {
        get(_name, fallback) {
          return fallback;
        }
      };
    },
    textDocuments: []
  }
};

const originalLoad = Module._load;
let EngDiagnosticsController;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") {
      return vscodeMock;
    }
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ EngDiagnosticsController } = require("../diagnosticsProvider"));
} finally {
  Module._load = originalLoad;
}

function documentFixture() {
  const lines = [
    "script main {",
    "    value := 1",
    "}"
  ];
  return {
    isDirty: false,
    languageId: "englang",
    lineCount: lines.length,
    uri: {
      fsPath: "C:\\workspace\\main.eng",
      toString() {
        return "file:///C:/workspace/main.eng";
      }
    },
    version: 1,
    getText() {
      return lines.join("\n");
    },
    lineAt(index) {
      return { text: lines[index] };
    }
  };
}

function diagnosticsCollection() {
  const deletes = [];
  const sets = [];
  return {
    deletes,
    sets,
    delete(uri) {
      deletes.push(uri.toString());
    },
    set(uri, diagnostics) {
      sets.push({ diagnostics, uri: uri.toString() });
    }
  };
}

function successfulReview() {
  return {
    format: "eng-ide-check-v1",
    diagnostics: [
      {
        code: "E-SCRIPT-001",
        severity: "error",
        message: "script blocks are not supported",
        help: "Move the body to top-level statements.",
        line: 1,
        column: 1
      },
      {
        code: "W-TABLE-LEGACY-SELECT-FIRST-ROW",
        severity: "warning",
        message: "legacy table selection",
        range: {
          start: { line: 1, character: 4 },
          end: { line: 1, character: 9 }
        }
      }
    ],
    review_document: {
      risks: []
    },
    semantic_tokens: {
      tokens: []
    }
  };
}

const originalExecFile = childProcess.execFile;
const processCalls = [];
childProcess.execFile = (runtime, args, options, callback) => {
  processCalls.push({ args, callback, options, runtime });
  return { stdin: undefined };
};

try {
  const document = documentFixture();
  const diagnostics = diagnosticsCollection();
  const cachedReviews = [];
  const clearedReviews = [];
  const riskDecorations = [];
  const validationDecorations = [];
  const semanticDecorations = [];
  const outputLines = [];
  const controller = new EngDiagnosticsController({}, diagnostics, {
    cacheReview(_document, review) {
      cachedReviews.push(review);
    },
    clearCachedReview(currentDocument) {
      clearedReviews.push(currentDocument.uri.toString());
    },
    diagnosticsRuntime() {
      return "eng-cli";
    },
    diagnosticsRuntimeLabel() {
      return "file diagnostics";
    },
    findRuntime() {
      return "C:\\fake\\eng.exe";
    },
    output: {
      appendLine(line) {
        outputLines.push(line);
      }
    },
    updateReviewRiskDecorations(_document, review) {
      riskDecorations.push(review);
    },
    updateReviewValidationDecorations(_document, review) {
      validationDecorations.push(review);
    },
    updateSemanticSymbolDecorations(_document, review) {
      semanticDecorations.push(review);
    },
    workspaceRoot() {
      return "C:\\workspace";
    }
  });

  controller.checkDocument(document);
  assert.strictEqual(processCalls.length, 1);
  assert.strictEqual(processCalls[0].runtime, "C:\\fake\\eng.exe");
  assert.deepStrictEqual(processCalls[0].args, [
    "ide-check",
    "C:\\workspace\\main.eng"
  ]);
  assert.strictEqual(processCalls[0].options.cwd, "C:\\workspace");
  assert.strictEqual(processCalls[0].options.maxBuffer, 10 * 1024 * 1024);

  const review = successfulReview();
  processCalls[0].callback(null, JSON.stringify(review), "");
  assert.strictEqual(diagnostics.sets.length, 1);
  assert.strictEqual(diagnostics.sets[0].uri, document.uri.toString());
  assert.strictEqual(diagnostics.sets[0].diagnostics.length, 2);

  const scriptDiagnostic = diagnostics.sets[0].diagnostics[0];
  assert.strictEqual(scriptDiagnostic.source, "eng/file");
  assert.strictEqual(scriptDiagnostic.severity, vscodeMock.DiagnosticSeverity.Error);
  assert.strictEqual(scriptDiagnostic.code.value, "E-SCRIPT-001");
  assert.match(scriptDiagnostic.code.target.toString(), /top_level_execution_policy/);
  assert.deepStrictEqual(scriptDiagnostic.range.start, { line: 0, character: 0 });
  assert.deepStrictEqual(scriptDiagnostic.range.end, { line: 0, character: 6 });
  assert.match(scriptDiagnostic.message, /Move the body to top-level statements/);
  assert.deepStrictEqual(scriptDiagnostic.tags, [vscodeMock.DiagnosticTag.Deprecated]);

  const legacyDiagnostic = diagnostics.sets[0].diagnostics[1];
  assert.strictEqual(legacyDiagnostic.source, "eng/file");
  assert.strictEqual(legacyDiagnostic.severity, vscodeMock.DiagnosticSeverity.Warning);
  assert.deepStrictEqual(legacyDiagnostic.range.start, { line: 1, character: 4 });
  assert.deepStrictEqual(legacyDiagnostic.range.end, { line: 1, character: 9 });
  assert.deepStrictEqual(legacyDiagnostic.tags, [vscodeMock.DiagnosticTag.Deprecated]);
  assert.deepStrictEqual(cachedReviews.at(-1), review);
  assert.strictEqual(riskDecorations.at(-1), cachedReviews.at(-1));
  assert.strictEqual(validationDecorations.at(-1), cachedReviews.at(-1));
  assert.strictEqual(semanticDecorations.at(-1), cachedReviews.at(-1));
  assert.ok(outputLines.some((line) => line.includes("Problems source: eng/file")));

  controller.checkDocument(document);
  assert.strictEqual(processCalls.length, 2);
  const toolError = Object.assign(new Error("fake eng.exe failed"), { code: 7 });
  processCalls[1].callback(toolError, "not editor json", "synthetic stderr");
  assert.strictEqual(diagnostics.sets.length, 2);
  const failureDiagnostic = diagnostics.sets.at(-1).diagnostics[0];
  assert.strictEqual(failureDiagnostic.source, "eng/file");
  assert.match(failureDiagnostic.message, /exit code 7/);
  assert.match(failureDiagnostic.message, /invalid JSON/);
  assert.match(failureDiagnostic.message, /synthetic stderr/);
  assert.match(failureDiagnostic.message, /englang\.runtimePath/);
  assert.match(failureDiagnostic.message, /EngLang output channel/);
  assert.deepStrictEqual(clearedReviews, [document.uri.toString()]);
  assert.strictEqual(riskDecorations.at(-1), undefined);
  assert.strictEqual(validationDecorations.at(-1), undefined);
  assert.strictEqual(semanticDecorations.at(-1), undefined);

  controller.checkDocument(document);
  assert.strictEqual(processCalls.length, 3);
  const setCountBeforeStaleResult = diagnostics.sets.length;
  document.version += 1;
  processCalls[2].callback(null, JSON.stringify(successfulReview()), "");
  assert.strictEqual(
    diagnostics.sets.length,
    setCountBeforeStaleResult,
    "saved-file diagnostics from an older document version must be discarded"
  );
} finally {
  childProcess.execFile = originalExecFile;
}

process.stdout.write("VS Code fake eng.exe diagnostics backend smoke passed.\n");

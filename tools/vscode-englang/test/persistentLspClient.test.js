"use strict";

const assert = require("assert");
const Module = require("module");
const { EventEmitter: NodeEventEmitter } = require("events");

class SemanticTokens {
  constructor(data, resultId) {
    this.data = data;
    this.resultId = resultId;
  }
}

class VscodeEventEmitter {
  constructor() {
    this.listeners = new Set();
    this.event = (listener) => {
      this.listeners.add(listener);
      return { dispose: () => this.listeners.delete(listener) };
    };
  }

  fire(value) {
    for (const listener of this.listeners) listener(value);
  }

  dispose() {
    this.listeners.clear();
  }
}

function fileUri(fsPath) {
  const normalized = fsPath.replace(/\\/g, "/");
  return {
    fsPath,
    scheme: "file",
    toString() {
      return `file:///${normalized.replace(/^\//, "")}`;
    }
  };
}

const workspaceFolder = {
  name: "workspace",
  uri: fileUri("C:\\workspace")
};
const workspaceMock = {
  textDocuments: [],
  workspaceFolders: [workspaceFolder],
  getConfiguration() {
    return {
      get(name, fallback) {
        return name === "liveDiagnosticsDelayMs" ? 125 : fallback;
      }
    };
  }
};
const vscodeMock = {
  EventEmitter: VscodeEventEmitter,
  SemanticTokens,
  Uri: { file: fileUri },
  workspace: workspaceMock
};

const originalLoad = Module._load;
let EngSemanticTokensProvider;
let PersistentLspClient;
let createPersistentLspRequests;
let incrementalContentChange;
try {
  Module._load = function loadWithVscodeMock(request, parent, isMain) {
    if (request === "vscode") return vscodeMock;
    return originalLoad.call(this, request, parent, isMain);
  };
  ({ EngSemanticTokensProvider } = require("../semanticTokensProvider"));
  ({
    PersistentLspClient,
    createPersistentLspRequests,
    incrementalContentChange
  } = require("../persistentLspClient"));
} finally {
  Module._load = originalLoad;
}

class FakeStream extends NodeEventEmitter {
  constructor(write) {
    super();
    this.writeHandler = write;
  }

  write(chunk) {
    this.writeHandler?.(Buffer.from(chunk));
    return true;
  }

  end() {
    this.emit("end");
  }
}

class FakeLspServer {
  constructor() {
    this.messages = [];
    this.input = Buffer.alloc(0);
    this.expectedLength = undefined;
    this.child = new NodeEventEmitter();
    this.child.stdin = new FakeStream((chunk) => this.receive(chunk));
    this.child.stdout = new FakeStream();
    this.child.stderr = new FakeStream();
    this.child.kill = () => this.close(1, "SIGTERM");
    this.closed = false;
  }

  receive(chunk) {
    this.input = Buffer.concat([this.input, chunk]);
    while (true) {
      if (this.expectedLength === undefined) {
        const headerEnd = this.input.indexOf("\r\n\r\n");
        if (headerEnd < 0) return;
        const header = this.input.subarray(0, headerEnd).toString("ascii");
        this.expectedLength = Number(/^content-length:\s*(\d+)$/im.exec(header)?.[1]);
        this.input = this.input.subarray(headerEnd + 4);
      }
      if (this.input.length < this.expectedLength) return;
      const body = this.input.subarray(0, this.expectedLength);
      this.input = this.input.subarray(this.expectedLength);
      this.expectedLength = undefined;
      const message = JSON.parse(body.toString("utf8"));
      this.messages.push(message);
      this.handle(message);
    }
  }

  handle(message) {
    switch (message.method) {
      case "initialize":
        this.respond(message.id, {
          capabilities: {
            experimental: { englangSnapshotProvider: true },
            semanticTokensProvider: { full: { delta: true }, range: true },
            textDocumentSync: { openClose: true, change: 2, save: { includeText: true } }
          }
        }, true);
        break;
      case "textDocument/didOpen":
      case "textDocument/didChange":
      case "textDocument/didSave":
        this.notify("textDocument/publishDiagnostics", {
          uri: message.params.textDocument.uri,
          version: message.params.textDocument.version,
          diagnostics: [{
            range: {
              start: { line: 0, character: 0 },
              end: { line: 0, character: 5 }
            },
            severity: 2,
            source: "eng",
            code: "W-TEST-001",
            message: "UTF-8 진단"
          }]
        });
        break;
      case "textDocument/semanticTokens/full":
        this.respond(message.id, { data: [0, 0, 5, 1, 3], resultId: "semantic-1" }, true);
        break;
      case "textDocument/hover":
        this.respond(message.id, {
          contents: { kind: "markdown", value: "**value**" }
        });
        break;
      case "textDocument/signatureHelp":
        this.respond(message.id, {
          signatures: [{
            label: "combine(left: Length [m], right: Length [m]) -> Length [m]",
            parameters: [
              { label: "left: Length [m]" },
              { label: "right: Length [m]" }
            ]
          }],
          activeSignature: 0,
          activeParameter: 1
        });
        break;
      case "englang/snapshot":
        this.respond(message.id, {
          format: "eng-lsp-snapshot-v1",
          diagnostics: [],
          semantic_tokens: { tokens: [] }
        });
        break;
      case "workspace/symbol":
        break;
      case "shutdown":
        this.respond(message.id, null);
        break;
      case "exit":
        this.close(0, null);
        break;
      default:
        break;
    }
  }

  respond(id, result, fragmented = false) {
    this.send({ jsonrpc: "2.0", id, result }, fragmented);
  }

  notify(method, params) {
    this.send({ jsonrpc: "2.0", method, params }, true);
  }

  send(message, fragmented = false) {
    const body = Buffer.from(JSON.stringify(message), "utf8");
    const frame = Buffer.concat([
      Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "ascii"),
      body
    ]);
    if (!fragmented) {
      this.child.stdout.emit("data", frame);
      return;
    }
    const split = Math.max(1, Math.floor(frame.length / 3));
    this.child.stdout.emit("data", frame.subarray(0, split));
    this.child.stdout.emit("data", frame.subarray(split, split * 2));
    this.child.stdout.emit("data", frame.subarray(split * 2));
  }

  close(code, signal) {
    if (this.closed) return;
    this.closed = true;
    this.child.emit("close", code, signal);
  }
}

function cancellationToken() {
  const listeners = new Set();
  return {
    isCancellationRequested: false,
    onCancellationRequested(listener) {
      listeners.add(listener);
      return { dispose: () => listeners.delete(listener) };
    },
    cancel() {
      this.isCancellationRequested = true;
      for (const listener of Array.from(listeners)) listener();
    }
  };
}

function wait(milliseconds = 0) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}

function incrementalChangeFixtures() {
  assert.deepStrictEqual(
    incrementalContentChange("value = 1", "value = 2"),
    {
      range: {
        start: { line: 0, character: 8 },
        end: { line: 0, character: 9 }
      },
      text: "2"
    }
  );
  assert.deepStrictEqual(
    incrementalContentChange('value = "😀" + 20\r\n', 'value = "😀" + 25\r\n'),
    {
      range: {
        start: { line: 0, character: 16 },
        end: { line: 0, character: 17 }
      },
      text: "5"
    }
  );
  assert.deepStrictEqual(
    incrementalContentChange("a = 1\r\nb = 2\r\n", "a = 1\r\nb = 20\r\n"),
    {
      range: {
        start: { line: 1, character: 5 },
        end: { line: 1, character: 5 }
      },
      text: "0"
    }
  );
  assert.deepStrictEqual(incrementalContentChange("a\r\n", "a\n"), { text: "a\n" });
  assert.deepStrictEqual(
    incrementalContentChange("unchanged", "unchanged"),
    {
      range: {
        start: { line: 0, character: 0 },
        end: { line: 0, character: 0 }
      },
      text: ""
    }
  );
}

async function main() {
  incrementalChangeFixtures();
  const document = {
    fileName: "C:\\workspace\\main.eng",
    isDirty: true,
    languageId: "englang",
    uri: fileUri("C:\\workspace\\main.eng"),
    version: 1,
    text: "value = 1",
    getText() {
      return this.text;
    }
  };
  workspaceMock.textDocuments = [document];

  const servers = [];
  const spawnCalls = [];
  const runtimeDocuments = [];
  const output = [];
  const client = new PersistentLspClient({
    extensionPath: "C:\\extension",
    extension: { packageJSON: { version: "0.1.0" } }
  }, {
    workspace: workspaceMock,
    isEngDocument: (candidate) => candidate.languageId === "englang",
    workspaceRoot: () => "C:\\workspace",
    findLspRuntime: (_context, runtimeDocument) => {
      runtimeDocuments.push(runtimeDocument);
      return "C:\\extension\\bin\\eng-lsp.exe";
    },
    findLspRuntimeForRoot: () => "C:\\extension\\bin\\eng-lsp.exe",
    semanticTokenTypes: ["keyword", "variable"],
    semanticTokenModifiers: ["declaration", "readonly"],
    restartDelayMs: 5,
    requestTimeoutMs: 1000,
    appendOutputLine: (line) => output.push(line),
    spawn(runtime, args, options) {
      const server = new FakeLspServer();
      servers.push(server);
      spawnCalls.push({ runtime, args, options });
      return server.child;
    }
  });

  const published = [];
  client.onDiagnostics((params) => published.push(params));
  await client.start({
    fileName: "C:\\workspace\\README.md",
    languageId: "markdown",
    uri: fileUri("C:\\workspace\\README.md")
  });
  assert.strictEqual(spawnCalls.length, 1);
  assert.strictEqual(runtimeDocuments[0], document);
  assert.deepStrictEqual(spawnCalls[0].args, ["--stdio"]);
  assert.strictEqual(spawnCalls[0].options.windowsHide, true);
  const initialized = servers[0].messages.find((message) => message.method === "initialize");
  assert.strictEqual(initialized.params.initializationOptions.diagnosticsDebounceMs, 125);
  assert.strictEqual(initialized.params.workspaceFolders[0].uri, workspaceFolder.uri.toString());
  assert.ok(servers[0].messages.some((message) => message.method === "initialized"));
  assert.strictEqual(
    servers[0].messages.filter((message) => message.method === "textDocument/didOpen").length,
    1
  );
  assert.strictEqual(published.at(-1).diagnostics[0].message, "UTF-8 진단");

  const fallbackCalls = [];
  const requests = createPersistentLspRequests({
    client,
    appendOutputLine: (line) => output.push(line),
    fallback: {
      clearSnapshotCache() {},
      snapshotDocumentSource() {
        fallbackCalls.push("snapshot");
      }
    }
  });
  const semantic = await requests.semanticTokensForDocument(document);
  assert.deepStrictEqual(semantic.data, [0, 0, 5, 1, 3]);
  const signatureHelp = await requests.signatureHelpForPosition(
    document,
    { line: 0, character: 9 }
  );
  assert.strictEqual(signatureHelp.activeParameter, 1);
  assert.ok(signatureHelp.signatures[0].label.startsWith("combine("));
  const signatureRequest = servers[0].messages.find(
    (message) => message.method === "textDocument/signatureHelp"
  );
  assert.deepStrictEqual(signatureRequest.params.position, { line: 0, character: 9 });
  const snapshot = await requests.snapshotDocumentSource(document, {});
  assert.strictEqual(snapshot.format, "eng-lsp-snapshot-v1");
  assert.deepStrictEqual(fallbackCalls, []);
  assert.deepStrictEqual(requests.clientStatus(), {
    state: "running",
    runtime: "C:\\extension\\bin\\eng-lsp.exe",
    protocol_semantic_tokens: true,
    snapshot_provider: true
  });

  document.version = 2;
  document.text = "value = 2";
  await client.changeDocument(document);
  const change = servers[0].messages.find((message) => message.method === "textDocument/didChange");
  assert.strictEqual(change.params.textDocument.version, 2);
  assert.deepStrictEqual(change.params.contentChanges, [{
    range: {
      start: { line: 0, character: 8 },
      end: { line: 0, character: 9 }
    },
    text: "2"
  }]);

  client.serverCapabilities.textDocumentSync.change = 1;
  document.version = 3;
  document.text = "value = 3";
  await client.changeDocument(document);
  const fullChange = servers[0].messages
    .filter((message) => message.method === "textDocument/didChange")
    .at(-1);
  assert.deepStrictEqual(fullChange.params.contentChanges, [{ text: "value = 3" }]);
  client.serverCapabilities.textDocumentSync.change = 2;

  await client.saveDocument(document);
  assert.ok(servers[0].messages.some((message) => message.method === "textDocument/didSave"));

  const token = cancellationToken();
  const pendingSymbols = client.requestWorkspace("workspace/symbol", { query: "value" }, token);
  await wait();
  token.cancel();
  assert.strictEqual(await pendingSymbols, undefined);
  const cancel = servers[0].messages.find((message) => message.method === "$/cancelRequest");
  assert.ok(Number.isInteger(cancel.params.id));

  let snapshotFallbacks = 0;
  const provider = new EngSemanticTokensProvider({}, {
    isEngDocument: () => true,
    semanticTokensForDocument: requests.semanticTokensForDocument,
    snapshotDocumentSource() {
      snapshotFallbacks += 1;
    }
  });
  const providerTokens = await provider.provideDocumentSemanticTokens(document, {});
  assert.deepStrictEqual(Array.from(providerTokens.data), [0, 0, 5, 1, 3]);
  assert.strictEqual(providerTokens.resultId, "semantic-1");
  assert.strictEqual(snapshotFallbacks, 0);
  provider.dispose();

  servers[0].close(7, null);
  await wait(25);
  assert.strictEqual(spawnCalls.length, 2);
  assert.ok(servers[1].messages.some((message) => message.method === "textDocument/didOpen"));
  assert.ok(output.some((line) => line.includes("will restart")));

  await client.stop();
  assert.ok(servers[1].messages.some((message) => message.method === "shutdown"));
  assert.ok(servers[1].messages.some((message) => message.method === "exit"));
  assert.strictEqual(client.status().state, "stopped");
  console.log("persistent LSP client smoke passed");
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});

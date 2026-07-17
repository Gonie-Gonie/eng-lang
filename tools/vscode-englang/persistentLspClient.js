"use strict";

const cp = require("child_process");
const nodePath = require("path");
const vscode = require("vscode");

const DEFAULT_REQUEST_TIMEOUT_MS = 30_000;
const INITIALIZE_TIMEOUT_MS = 10_000;
const SHUTDOWN_TIMEOUT_MS = 1_000;
const MAX_HEADER_BYTES = 8 * 1024;
const MAX_MESSAGE_BYTES = 100 * 1024 * 1024;

class LspResponseError extends Error {
  constructor(method, response) {
    super(response?.message || `EngLang LSP request failed: ${method}`);
    this.name = "LspResponseError";
    this.code = response?.code;
    this.data = response?.data;
    this.method = method;
  }
}

class LspConnectionError extends Error {
  constructor(message) {
    super(message);
    this.name = "LspConnectionError";
  }
}

class PersistentLspClient {
  constructor(context, options = {}) {
    this.context = context;
    this.workspace = options.workspace ?? vscode.workspace;
    this.isEngDocument = options.isEngDocument ?? (() => true);
    this.findLspRuntime = options.findLspRuntime;
    this.findLspRuntimeForRoot = options.findLspRuntimeForRoot;
    this.workspaceRoot = options.workspaceRoot;
    this.appendOutputLine = options.appendOutputLine ?? (() => undefined);
    this.spawn = options.spawn ?? cp.spawn;
    this.semanticTokenTypes = options.semanticTokenTypes ?? [];
    this.semanticTokenModifiers = options.semanticTokenModifiers ?? [];
    this.requestTimeoutMs = options.requestTimeoutMs ?? DEFAULT_REQUEST_TIMEOUT_MS;
    this.restartDelayMs = options.restartDelayMs ?? 500;
    this.maxMessageBytes = options.maxMessageBytes ?? MAX_MESSAGE_BYTES;
    this.state = "stopped";
    this.child = undefined;
    this.startPromise = undefined;
    this.stopPromise = undefined;
    this.closePromise = undefined;
    this.resolveClose = undefined;
    this.restartTimer = undefined;
    this.intentionalStop = false;
    this.disposed = false;
    this.generation = 0;
    this.nextRequestId = 1;
    this.pendingRequests = new Map();
    this.syncedDocuments = new Map();
    this.diagnosticsListeners = new Set();
    this.stdoutBuffer = Buffer.alloc(0);
    this.expectedBodyLength = undefined;
    this.serverCapabilities = {};
  }

  onDiagnostics(listener) {
    this.diagnosticsListeners.add(listener);
    return {
      dispose: () => this.diagnosticsListeners.delete(listener)
    };
  }

  status() {
    return {
      state: this.state,
      runtime: this.runtime,
      serverCapabilities: this.serverCapabilities
    };
  }

  async start(document) {
    if (this.disposed) {
      throw new LspConnectionError("EngLang language client is disposed");
    }
    if (this.state === "running") {
      return;
    }
    if (this.state === "starting" && this.startPromise) {
      return this.startPromise;
    }
    if (this.state === "stopping" && this.stopPromise) {
      await this.stopPromise;
    }

    this.clearRestartTimer();
    this.intentionalStop = false;
    this.state = "starting";
    this.stdoutBuffer = Buffer.alloc(0);
    this.expectedBodyLength = undefined;
    this.syncedDocuments.clear();
    const generation = ++this.generation;
    const anchorDocument = document && this.isEngDocument(document)
      ? document
      : this.openEngDocuments()[0];
    const root = this.initialWorkspaceRoot(anchorDocument);
    const runtime = this.resolveRuntime(anchorDocument, root);
    this.runtime = runtime;

    let child;
    try {
      child = this.spawn(runtime, ["--stdio"], {
        cwd: root || this.context.extensionPath,
        stdio: ["pipe", "pipe", "pipe"],
        windowsHide: true
      });
    } catch (error) {
      this.state = "stopped";
      throw new LspConnectionError(`Unable to start ${runtime}: ${error.message}`);
    }
    this.child = child;
    this.closePromise = new Promise((resolve) => {
      this.resolveClose = resolve;
    });
    this.attachProcess(child, generation);

    const startPromise = this.sendRequest("initialize", this.initializeParams(anchorDocument, root), undefined, {
      allowStarting: true,
      timeoutMs: INITIALIZE_TIMEOUT_MS
    }).then((result) => {
      if (
        generation !== this.generation
        || this.child !== child
        || this.intentionalStop
        || this.disposed
        || this.state === "stopping"
      ) {
        throw new LspConnectionError("EngLang language server changed while initializing");
      }
      this.serverCapabilities = result?.capabilities ?? {};
      this.state = "running";
      this.sendNotification("initialized", {}, { allowStarting: true });
      for (const openDocument of this.openEngDocuments()) {
        this.syncDocumentNow(openDocument);
      }
      this.appendOutputLine(`Persistent EngLang language server started: ${runtime}`);
    }).catch((error) => {
      if (generation === this.generation && this.child === child) {
        if (this.intentionalStop || this.disposed || this.state === "stopping") {
          child.kill?.();
          this.handleProcessClose(child, generation, null, null);
          return;
        }
        this.failProcess(child, generation, error);
      }
      throw error;
    }).finally(() => {
      if (this.startPromise === startPromise) {
        this.startPromise = undefined;
      }
    });
    this.startPromise = startPromise;
    return startPromise;
  }

  async restart(document) {
    await this.stop();
    if (!this.disposed) {
      await this.start(document);
    }
  }

  async stop() {
    if (this.state === "stopped") {
      return;
    }
    if (this.state === "stopping" && this.stopPromise) {
      return this.stopPromise;
    }

    this.intentionalStop = true;
    this.clearRestartTimer();
    const child = this.child;
    const generation = this.generation;
    const wasRunning = this.state === "running";
    this.state = "stopping";
    const stopPromise = (async () => {
      if (child && wasRunning) {
        try {
          await this.sendRequest("shutdown", null, undefined, {
            allowStarting: true,
            timeoutMs: SHUTDOWN_TIMEOUT_MS
          });
        } catch (error) {
          this.appendOutputLine(`EngLang language server shutdown warning: ${error.message}`);
        }
        if (this.child === child) {
          this.sendNotification("exit", undefined, { allowStarting: true });
          child.stdin?.end?.();
        }
      }
      await Promise.race([this.closePromise ?? Promise.resolve(), delay(SHUTDOWN_TIMEOUT_MS)]);
      if (this.child === child) {
        child?.kill?.();
        this.handleProcessClose(child, generation, null, null);
      }
    })().finally(() => {
      if (this.stopPromise === stopPromise) {
        this.stopPromise = undefined;
      }
    });
    this.stopPromise = stopPromise;
    return stopPromise;
  }

  dispose() {
    this.disposed = true;
    this.diagnosticsListeners.clear();
    void this.stop();
  }

  async openDocument(document) {
    if (!this.isEngDocument(document)) {
      return;
    }
    await this.start(document);
    this.syncDocumentNow(document);
  }

  async changeDocument(document) {
    if (!this.isEngDocument(document)) {
      return;
    }
    await this.start(document);
    this.syncDocumentNow(document);
  }

  async saveDocument(document) {
    if (!this.isEngDocument(document)) {
      return;
    }
    await this.ensureDocument(document);
    const uri = document.uri.toString();
    this.sendNotification("textDocument/didSave", {
      textDocument: { uri, version: document.version },
      text: document.getText()
    });
  }

  async closeDocument(document) {
    const uri = document?.uri?.toString?.();
    if (!uri) {
      return;
    }
    if (this.state === "starting" && this.startPromise) {
      try {
        await this.startPromise;
      } catch {
        return;
      }
    }
    if (this.state === "running" && this.syncedDocuments.has(uri)) {
      this.sendNotification("textDocument/didClose", { textDocument: { uri } });
    }
    this.syncedDocuments.delete(uri);
  }

  async watchedFileChanged(uri, type = 2) {
    if (!uri) {
      return;
    }
    await this.start();
    this.sendNotification("workspace/didChangeWatchedFiles", {
      changes: [{ uri: uri.toString(), type }]
    });
  }

  async requestDocument(method, document, params = {}, cancellationToken) {
    await this.ensureDocument(document);
    return this.sendRequest(method, {
      ...params,
      textDocument: {
        ...(params.textDocument ?? {}),
        uri: document.uri.toString()
      }
    }, cancellationToken);
  }

  async requestWorkspace(method, params = {}, cancellationToken, document) {
    await this.start(document);
    for (const openDocument of this.openEngDocuments()) {
      this.syncDocumentNow(openDocument);
    }
    return this.sendRequest(method, params, cancellationToken);
  }

  async ensureDocument(document) {
    if (!document || !this.isEngDocument(document)) {
      throw new LspConnectionError("EngLang LSP request requires an EngLang document");
    }
    await this.start(document);
    this.syncDocumentNow(document);
  }

  syncDocumentNow(document) {
    if (!document || !this.isEngDocument(document) || this.state === "stopped") {
      return;
    }
    const uri = document.uri.toString();
    const text = document.getText();
    const version = Number.isInteger(document.version) ? document.version : 0;
    const current = this.syncedDocuments.get(uri);
    if (!current) {
      this.syncedDocuments.set(uri, { text, version });
      this.sendNotification("textDocument/didOpen", {
        textDocument: {
          uri,
          languageId: document.languageId || "englang",
          version,
          text
        }
      }, { allowStarting: true });
      return;
    }
    if (current.version === version && current.text === text) {
      return;
    }
    if (version <= current.version) {
      this.appendOutputLine(`Skipped stale EngLang document sync for ${uri} at version ${version}`);
      return;
    }
    this.syncedDocuments.set(uri, { text, version });
    this.sendNotification("textDocument/didChange", {
      textDocument: { uri, version },
      contentChanges: [{ text }]
    }, { allowStarting: true });
  }

  sendRequest(method, params, cancellationToken, options = {}) {
    if (cancellationToken?.isCancellationRequested) {
      return Promise.resolve(undefined);
    }
    const id = this.nextRequestId++;
    const key = String(id);
    const timeoutMs = options.timeoutMs ?? this.requestTimeoutMs;
    return new Promise((resolve, reject) => {
      let cancellationSubscription;
      const timer = setTimeout(() => {
        if (!this.pendingRequests.has(key)) {
          return;
        }
        this.tryCancelRequest(id);
        this.settlePendingRequest(key, "reject", new LspConnectionError(
          `EngLang LSP ${method} timed out after ${timeoutMs} ms`
        ));
      }, timeoutMs);
      timer.unref?.();
      this.pendingRequests.set(key, {
        id,
        method,
        resolve,
        reject,
        timer,
        get cancellationSubscription() {
          return cancellationSubscription;
        }
      });
      cancellationSubscription = cancellationToken?.onCancellationRequested?.(() => {
        if (!this.pendingRequests.has(key)) {
          return;
        }
        this.tryCancelRequest(id);
        this.settlePendingRequest(key, "resolve", undefined);
      });
      try {
        if (this.pendingRequests.has(key)) {
          this.writeMessage({ jsonrpc: "2.0", id, method, params }, options);
        }
      } catch (error) {
        this.settlePendingRequest(key, "reject", error);
      }
    });
  }

  sendNotification(method, params, options = {}) {
    const message = { jsonrpc: "2.0", method };
    if (params !== undefined) {
      message.params = params;
    }
    this.writeMessage(message, options);
  }

  tryCancelRequest(id) {
    try {
      this.sendNotification("$/cancelRequest", { id }, { allowStarting: true });
    } catch {
      // Process teardown will reject or discard the matching pending request.
    }
  }

  writeMessage(message, options = {}) {
    if (
      !this.child?.stdin?.write
      || (this.state !== "running" && !(options.allowStarting && ["starting", "stopping"].includes(this.state)))
    ) {
      throw new LspConnectionError("EngLang language server is not running");
    }
    const body = Buffer.from(JSON.stringify(message), "utf8");
    if (body.length > this.maxMessageBytes) {
      throw new LspConnectionError("EngLang LSP request exceeded the message size limit");
    }
    const header = Buffer.from(`Content-Length: ${body.length}\r\n\r\n`, "ascii");
    this.child.stdin.write(Buffer.concat([header, body]));
  }

  attachProcess(child, generation) {
    child.stdin?.on?.("error", (error) => this.failProcess(child, generation, error));
    child.stdout?.on?.("data", (chunk) => this.handleStdout(child, generation, chunk));
    child.stdout?.on?.("error", (error) => this.failProcess(child, generation, error));
    child.stderr?.on?.("data", (chunk) => {
      const text = String(chunk ?? "").trim();
      if (text) {
        this.appendOutputLine(text);
      }
    });
    child.on?.("error", (error) => this.failProcess(child, generation, error));
    child.on?.("close", (code, signal) => this.handleProcessClose(child, generation, code, signal));
  }

  handleStdout(child, generation, chunk) {
    if (this.child !== child || generation !== this.generation) {
      return;
    }
    this.stdoutBuffer = Buffer.concat([this.stdoutBuffer, Buffer.from(chunk)]);
    try {
      while (true) {
        if (this.expectedBodyLength === undefined) {
          const headerEnd = this.stdoutBuffer.indexOf("\r\n\r\n");
          if (headerEnd < 0) {
            if (this.stdoutBuffer.length > MAX_HEADER_BYTES) {
              throw new LspConnectionError("EngLang LSP response header exceeded the size limit");
            }
            return;
          }
          const header = this.stdoutBuffer.subarray(0, headerEnd).toString("ascii");
          const match = /^content-length:\s*(\d+)\s*$/im.exec(header);
          if (!match) {
            throw new LspConnectionError("EngLang LSP response is missing Content-Length");
          }
          this.expectedBodyLength = Number(match[1]);
          if (!Number.isSafeInteger(this.expectedBodyLength) || this.expectedBodyLength > this.maxMessageBytes) {
            throw new LspConnectionError("EngLang LSP response exceeded the message size limit");
          }
          this.stdoutBuffer = this.stdoutBuffer.subarray(headerEnd + 4);
        }
        if (this.stdoutBuffer.length < this.expectedBodyLength) {
          return;
        }
        const body = this.stdoutBuffer.subarray(0, this.expectedBodyLength);
        this.stdoutBuffer = this.stdoutBuffer.subarray(this.expectedBodyLength);
        this.expectedBodyLength = undefined;
        this.handleMessage(JSON.parse(body.toString("utf8")));
      }
    } catch (error) {
      this.failProcess(child, generation, error);
    }
  }

  handleMessage(message) {
    if (Object.prototype.hasOwnProperty.call(message ?? {}, "id")) {
      const key = String(message.id);
      const pending = this.pendingRequests.get(key);
      if (!pending) {
        return;
      }
      if (message.error) {
        this.settlePendingRequest(key, "reject", new LspResponseError(pending.method, message.error));
      } else {
        this.settlePendingRequest(key, "resolve", message.result);
      }
      return;
    }
    if (message?.method === "textDocument/publishDiagnostics") {
      for (const listener of this.diagnosticsListeners) {
        try {
          listener(message.params ?? {});
        } catch (error) {
          this.appendOutputLine(`EngLang diagnostics listener failed: ${error.message}`);
        }
      }
    }
  }

  settlePendingRequest(key, outcome, value) {
    const pending = this.pendingRequests.get(key);
    if (!pending) {
      return;
    }
    this.pendingRequests.delete(key);
    clearTimeout(pending.timer);
    pending.cancellationSubscription?.dispose?.();
    pending[outcome](value);
  }

  failProcess(child, generation, error) {
    if (this.child !== child || generation !== this.generation) {
      return;
    }
    const failure = error instanceof Error
      ? error
      : new LspConnectionError(String(error ?? "EngLang language server failed"));
    this.appendOutputLine(`Persistent EngLang language server failed: ${failure.message}`);
    child.kill?.();
    this.handleProcessClose(child, generation, null, null, failure);
  }

  handleProcessClose(child, generation, code, signal, failure) {
    if (this.child !== child || generation !== this.generation) {
      return;
    }
    const wasIntentional = this.intentionalStop || this.disposed || this.state === "stopping";
    const detail = failure ?? new LspConnectionError(
      `EngLang language server exited${code !== null && code !== undefined ? ` with code ${code}` : ""}${signal ? ` (${signal})` : ""}`
    );
    for (const key of Array.from(this.pendingRequests.keys())) {
      this.settlePendingRequest(key, "reject", detail);
    }
    this.child = undefined;
    this.state = "stopped";
    this.startPromise = undefined;
    this.syncedDocuments.clear();
    this.stdoutBuffer = Buffer.alloc(0);
    this.expectedBodyLength = undefined;
    this.serverCapabilities = {};
    this.resolveClose?.();
    this.resolveClose = undefined;
    this.closePromise = undefined;
    if (!wasIntentional) {
      this.scheduleRestart();
    }
  }

  scheduleRestart() {
    if (this.disposed || this.restartTimer || this.openEngDocuments().length === 0) {
      return;
    }
    this.appendOutputLine(`EngLang language server will restart in ${this.restartDelayMs} ms`);
    this.restartTimer = setTimeout(() => {
      this.restartTimer = undefined;
      this.start().catch((error) => {
        this.appendOutputLine(`EngLang language server restart failed: ${error.message}`);
        this.scheduleRestart();
      });
    }, this.restartDelayMs);
    this.restartTimer.unref?.();
  }

  clearRestartTimer() {
    if (this.restartTimer) {
      clearTimeout(this.restartTimer);
      this.restartTimer = undefined;
    }
  }

  initializeParams(document, root) {
    const folders = (this.workspace.workspaceFolders ?? [])
      .filter((folder) => folder?.uri)
      .map((folder) => ({
        uri: folder.uri.toString(),
        name: folder.name || nodePath.basename(folder.uri.fsPath || folder.uri.toString())
      }));
    const rootUri = folders[0]?.uri
      ?? (root && vscode.Uri?.file ? vscode.Uri.file(root).toString() : null);
    const delayMs = Number(
      this.workspace.getConfiguration?.("englang", document?.uri)?.get?.("liveDiagnosticsDelayMs", 350)
    );
    return {
      processId: process.pid,
      clientInfo: {
        name: "englang-vscode",
        version: this.context.extension?.packageJSON?.version ?? "unknown"
      },
      rootUri,
      workspaceFolders: folders,
      capabilities: {
        workspace: {
          workspaceFolders: true
        },
        textDocument: {
          publishDiagnostics: { versionSupport: true },
          semanticTokens: {
            requests: { range: true, full: { delta: true } },
            tokenTypes: this.semanticTokenTypes,
            tokenModifiers: this.semanticTokenModifiers,
            formats: ["relative"],
            overlappingTokenSupport: false,
            multilineTokenSupport: false
          }
        }
      },
      initializationOptions: {
        diagnosticsDebounceMs: Number.isFinite(delayMs)
          ? Math.max(0, Math.min(5000, Math.trunc(delayMs)))
          : 350
      }
    };
  }

  initialWorkspaceRoot(document) {
    if (document) {
      return this.workspaceRoot?.(document) || nodePath.dirname(document.uri.fsPath);
    }
    return this.workspace.workspaceFolders?.find((folder) => folder.uri?.scheme === "file")?.uri?.fsPath
      ?? this.context.extensionPath;
  }

  resolveRuntime(document, root) {
    if (document && this.findLspRuntime) {
      return this.findLspRuntime(this.context, document);
    }
    return this.findLspRuntimeForRoot?.(this.context, root, document) ?? "eng-lsp.exe";
  }

  openEngDocuments() {
    return (this.workspace.textDocuments ?? []).filter((document) => this.isEngDocument(document));
  }
}

function createPersistentLspRequests(options = {}) {
  const client = options.client;
  const fallback = options.fallback ?? {};
  const appendOutputLine = options.appendOutputLine ?? (() => undefined);

  async function callProtocol(label, request, fallbackName, fallbackArgs = []) {
    try {
      return await request();
    } catch (error) {
      appendOutputLine(`Persistent EngLang ${label} failed: ${error.message}`);
      if (typeof fallback[fallbackName] === "function") {
        appendOutputLine(`Using compatibility ${fallbackName} request for this operation.`);
        return fallback[fallbackName](...fallbackArgs);
      }
      return undefined;
    }
  }

  function snapshotDocumentSource(document, context, cancellationToken) {
    return callProtocol(
      "snapshot request",
      async () => {
        const snapshot = await client.requestDocument(
          "englang/snapshot",
          document,
          {},
          cancellationToken
        );
        if (snapshot?.format !== "eng-lsp-snapshot-v1") {
          throw new LspConnectionError("language server returned an invalid editor snapshot");
        }
        return snapshot;
      },
      "snapshotDocumentSource",
      [document, context, cancellationToken]
    );
  }

  function completionSnapshotForPosition(document, position, context, cancellationToken) {
    return callProtocol(
      "completion request",
      async () => {
        const result = await client.requestDocument(
          "textDocument/completion",
          document,
          { position: lspPosition(position) },
          cancellationToken
        );
        const completions = Array.isArray(result) ? result : result?.items;
        return { completions: Array.isArray(completions) ? completions : [] };
      },
      "completionSnapshotForPosition",
      [document, position, context, cancellationToken]
    );
  }

  function definitionSnapshotForPosition(document, position, context, cancellationToken) {
    return callProtocol(
      "definition request",
      () => client.requestDocument(
        "textDocument/definition",
        document,
        { position: lspPosition(position) },
        cancellationToken
      ),
      "definitionSnapshotForPosition",
      [document, position, context, cancellationToken]
    );
  }

  function documentHighlightsForPosition(document, position, context, cancellationToken) {
    return callProtocol(
      "document highlight request",
      () => client.requestDocument(
        "textDocument/documentHighlight",
        document,
        { position: lspPosition(position) },
        cancellationToken
      ),
      "documentHighlightsForPosition",
      [document, position, context, cancellationToken]
    );
  }

  function referencesForPosition(
    document,
    position,
    includeDeclaration,
    context,
    cancellationToken
  ) {
    return callProtocol(
      "references request",
      () => client.requestDocument(
        "textDocument/references",
        document,
        {
          position: lspPosition(position),
          context: { includeDeclaration: includeDeclaration !== false }
        },
        cancellationToken
      ),
      "referencesForPosition",
      [document, position, includeDeclaration, context, cancellationToken]
    );
  }

  async function prepareRenameForPosition(document, position, context, cancellationToken) {
    try {
      return await client.requestDocument(
        "textDocument/prepareRename",
        document,
        { position: lspPosition(position) },
        cancellationToken
      );
    } catch (error) {
      if (error instanceof LspResponseError) {
        return { error: error.message };
      }
      return callProtocol(
        "rename preparation request",
        () => Promise.reject(error),
        "prepareRenameForPosition",
        [document, position, context, cancellationToken]
      );
    }
  }

  async function renameForPosition(document, position, newName, context, cancellationToken) {
    try {
      return await client.requestDocument(
        "textDocument/rename",
        document,
        { position: lspPosition(position), newName },
        cancellationToken
      );
    } catch (error) {
      if (error instanceof LspResponseError) {
        return { error: error.message };
      }
      return callProtocol(
        "rename request",
        () => Promise.reject(error),
        "renameForPosition",
        [document, position, newName, context, cancellationToken]
      );
    }
  }

  function workspaceSymbolsForQuery(query, context, cancellationToken) {
    return callProtocol(
      "workspace symbol request",
      async () => {
        const result = await client.requestWorkspace(
          "workspace/symbol",
          { query: query ?? "" },
          cancellationToken
        );
        return Array.isArray(result) ? result : [];
      },
      "workspaceSymbolsForQuery",
      [query, context, cancellationToken]
    );
  }

  function hoverForPosition(document, position, cancellationToken) {
    return callProtocol(
      "hover request",
      () => client.requestDocument(
        "textDocument/hover",
        document,
        { position: lspPosition(position) },
        cancellationToken
      )
    );
  }

  function semanticTokensForDocument(document, cancellationToken) {
    return callProtocol(
      "semantic token request",
      () => client.requestDocument(
        "textDocument/semanticTokens/full",
        document,
        {},
        cancellationToken
      )
    );
  }

  function documentSymbolsForDocument(document, cancellationToken) {
    return callProtocol(
      "document symbol request",
      () => client.requestDocument(
        "textDocument/documentSymbol",
        document,
        {},
        cancellationToken
      )
    );
  }

  function foldingRangesForDocument(document, cancellationToken) {
    return callProtocol(
      "folding range request",
      () => client.requestDocument(
        "textDocument/foldingRange",
        document,
        {},
        cancellationToken
      )
    );
  }

  function formattingEditsForDocument(document, formattingOptions, cancellationToken) {
    return callProtocol(
      "document formatting request",
      () => client.requestDocument(
        "textDocument/formatting",
        document,
        { options: lspFormattingOptions(formattingOptions) },
        cancellationToken
      )
    );
  }

  function rangeFormattingEditsForDocument(document, range, formattingOptions, cancellationToken) {
    return callProtocol(
      "range formatting request",
      () => client.requestDocument(
        "textDocument/rangeFormatting",
        document,
        {
          range: lspRange(range),
          options: lspFormattingOptions(formattingOptions)
        },
        cancellationToken
      )
    );
  }

  function codeActionsForDocumentRange(
    document,
    range,
    diagnostics,
    cancellationToken
  ) {
    return callProtocol(
      "code action request",
      () => client.requestDocument(
        "textDocument/codeAction",
        document,
        {
          range: lspRange(range),
          context: {
            diagnostics: (diagnostics ?? []).map(lspDiagnostic),
            only: ["quickfix"]
          }
        },
        cancellationToken
      )
    );
  }

  function clearSnapshotCache(document) {
    fallback.clearSnapshotCache?.(document);
  }

  function clientStatus() {
    const status = client.status();
    return {
      state: status.state,
      runtime: status.runtime ?? null,
      protocol_semantic_tokens: Boolean(status.serverCapabilities?.semanticTokensProvider),
      snapshot_provider: status.serverCapabilities?.experimental?.englangSnapshotProvider === true
    };
  }

  return {
    clearSnapshotCache,
    clientStatus,
    snapshotDocumentSource,
    workspaceSymbolsForQuery,
    completionSnapshotForPosition,
    definitionSnapshotForPosition,
    documentHighlightsForPosition,
    referencesForPosition,
    prepareRenameForPosition,
    renameForPosition,
    formatDocumentSource: fallback.formatDocumentSource,
    codeActionsForDocumentSource: fallback.codeActionsForDocumentSource,
    hoverForPosition,
    semanticTokensForDocument,
    documentSymbolsForDocument,
    foldingRangesForDocument,
    formattingEditsForDocument,
    rangeFormattingEditsForDocument,
    codeActionsForDocumentRange
  };
}

function lspPosition(position) {
  return {
    line: Number(position?.line ?? 0),
    character: Number(position?.character ?? 0)
  };
}

function lspRange(range) {
  return {
    start: lspPosition(range?.start),
    end: lspPosition(range?.end)
  };
}

function lspFormattingOptions(options) {
  return {
    tabSize: Number.isInteger(options?.tabSize) ? options.tabSize : 2,
    insertSpaces: options?.insertSpaces !== false,
    trimTrailingWhitespace: options?.trimTrailingWhitespace === true,
    insertFinalNewline: options?.insertFinalNewline === true,
    trimFinalNewlines: options?.trimFinalNewlines === true
  };
}

function lspDiagnostic(diagnostic) {
  const code = typeof diagnostic?.code === "string"
    ? diagnostic.code
    : diagnostic?.code?.value;
  const result = {
    range: lspRange(diagnostic?.range),
    severity: Number.isInteger(diagnostic?.severity) ? diagnostic.severity + 1 : undefined,
    source: diagnostic?.source,
    message: diagnostic?.message ?? ""
  };
  if (code !== undefined) {
    result.code = code;
  }
  if (Array.isArray(diagnostic?.tags) && diagnostic.tags.length > 0) {
    result.tags = diagnostic.tags;
  }
  return result;
}

function delay(milliseconds) {
  return new Promise((resolve) => {
    const timer = setTimeout(resolve, milliseconds);
    timer.unref?.();
  });
}

module.exports = {
  LspConnectionError,
  LspResponseError,
  PersistentLspClient,
  createPersistentLspRequests,
  lspDiagnostic,
  lspFormattingOptions,
  lspPosition,
  lspRange
};

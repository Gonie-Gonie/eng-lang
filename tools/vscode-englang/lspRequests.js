const cp = require("child_process");
const nodePath = require("path");
const vscode = require("vscode");

function createLspRequests(options = {}) {
  const isEngDocument = options.isEngDocument ?? (() => true);
  const findLspRuntime = options.findLspRuntime;
  const findLspRuntimeForRoot = options.findLspRuntimeForRoot;
  const workspaceRoot = options.workspaceRoot;
  const appendOutputLine = options.appendOutputLine ?? (() => undefined);
  const snapshotPromiseCache = new Map();
  const snapshotResultCache = new Map();
  const snapshotResultTtlMs = options.snapshotResultTtlMs ?? 2000;

  function clearSnapshotCache(document) {
    const key = snapshotCacheKey(document);
    snapshotPromiseCache.delete(key);
    snapshotResultCache.delete(key);
  }

  function snapshotDocumentSource(document, context, cancellationToken) {
    if (cancellationToken?.isCancellationRequested) {
      return Promise.resolve(undefined);
    }
    const key = snapshotCacheKey(document);
    const cachedResult = snapshotResultCache.get(key);
    if (cachedResult && cachedResult.expiresAt > Date.now()) {
      return snapshotResultForCaller(Promise.resolve(cachedResult.value), cancellationToken);
    }
    if (cachedResult) {
      snapshotResultCache.delete(key);
    }
    const cached = snapshotPromiseCache.get(key);
    if (cached) {
      return snapshotResultForCaller(cached, cancellationToken);
    }

    const promise = new Promise((resolve) => {
      const runtime = findLspRuntime(context, document);
      const cwd = workspaceRoot(document);
      const documentVersion = document.version;
      const documentText = document.getText();
      let settled = false;
      const finish = (value) => {
        if (settled) {
          return;
        }
        settled = true;
        if (document.version !== documentVersion) {
          snapshotPromiseCache.delete(key);
          snapshotResultCache.delete(key);
          resolve(undefined);
          return;
        }
        if (value) {
          snapshotResultCache.set(key, {
            value,
            expiresAt: Date.now() + snapshotResultTtlMs
          });
        }
        resolve(value);
      };

      const child = cp.execFile(
        runtime,
        ["--snapshot-stdin", document.uri.fsPath],
        { cwd, maxBuffer: 10 * 1024 * 1024 },
        (error, stdout, stderr) => {
          if (stderr && stderr.trim().length > 0) {
            appendOutputLine(stderr.trim());
          }
          if (error) {
            appendOutputLine(`Live editor check failed: ${error.message}`);
            finish(undefined);
            return;
          }
          try {
            finish(JSON.parse(stdout));
          } catch (parseError) {
            appendOutputLine(`Unable to parse EngLang live editor data: ${parseError.message}`);
            finish(undefined);
          }
        }
      );

      if (child.stdin) {
        child.stdin.end(documentText);
      }
    });
    snapshotPromiseCache.set(key, promise);
    promise.finally(() => {
      if (snapshotPromiseCache.get(key) === promise) {
        snapshotPromiseCache.delete(key);
      }
    });
    return snapshotResultForCaller(promise, cancellationToken);
  }

  function snapshotResultForCaller(promise, cancellationToken) {
    if (!cancellationToken) {
      return promise;
    }
    if (cancellationToken.isCancellationRequested) {
      return Promise.resolve(undefined);
    }
    return new Promise((resolve, reject) => {
      let settled = false;
      let cancellationSubscription;
      const finish = (callback, value) => {
        if (settled) {
          return;
        }
        settled = true;
        cancellationSubscription?.dispose?.();
        callback(value);
      };
      cancellationSubscription = cancellationToken.onCancellationRequested(() => {
        finish(resolve, undefined);
      });
      if (settled) {
        cancellationSubscription?.dispose?.();
        cancellationSubscription = undefined;
      }
      promise.then(
        (value) => finish(resolve, value),
        (error) => finish(reject, error)
      );
    });
  }

  function snapshotCacheKey(document) {
    return `${document.uri.toString()}@${document.version}`;
  }

  async function workspaceSymbolsForQuery(query, context, cancellationToken) {
    const folders = (vscode.workspace.workspaceFolders ?? [])
      .filter((folder) => folder.uri.scheme === "file");
    if (folders.length === 0 || cancellationToken?.isCancellationRequested) {
      return [];
    }

    const results = await Promise.all(
      folders.map((folder) => workspaceSymbolsForFolder(folder, query, context, cancellationToken))
    );
    const seen = new Set();
    const symbols = [];
    for (const symbol of results.flat()) {
      const uri = symbol?.location?.uri ?? "";
      const line = symbol?.location?.range?.start?.line ?? 0;
      const key = `${symbol?.name ?? ""}\n${uri}\n${line}`;
      if (!symbol?.name || seen.has(key)) {
        continue;
      }
      seen.add(key);
      symbols.push(symbol);
    }
    return symbols;
  }

  function workspaceSymbolsForFolder(folder, query, context, cancellationToken) {
    return new Promise((resolve) => {
      if (cancellationToken?.isCancellationRequested) {
        resolve([]);
        return;
      }

      const root = folder.uri.fsPath;
      const runtime = findLspRuntimeForRoot(context, root);
      const openDocuments = workspaceOpenDocumentsForFolder(folder);
      const documentStates = openDocuments.map((document) => ({
        dirty: document.isDirty,
        uri: document.uri.toString(),
        version: document.version
      }));
      const payload = JSON.stringify({
        format: "eng-lsp-open-documents-v1",
        documents: openDocuments
          .filter((document) => document.isDirty)
          .map((document) => ({
            path: document.uri.fsPath,
            source: document.getText()
          }))
      });
      let settled = false;
      let cancellationSubscription;
      const finish = (value) => {
        if (settled) {
          return;
        }
        settled = true;
        cancellationSubscription?.dispose?.();
        resolve(value);
      };

      const child = cp.execFile(
        runtime,
        ["--workspace-symbols-stdin", root, query ?? ""],
        { cwd: root, maxBuffer: 10 * 1024 * 1024 },
        (error, stdout, stderr) => {
          if (settled) {
            return;
          }
          if (stderr && stderr.trim().length > 0) {
            appendOutputLine(stderr.trim());
          }
          if (error) {
            appendOutputLine(`workspace symbol lookup failed: ${error.message}`);
            finish([]);
            return;
          }
          if (!workspaceDocumentStatesAreCurrent(folder, documentStates)) {
            finish([]);
            return;
          }
          try {
            const payload = JSON.parse(stdout);
            if (payload?.format !== "eng-lsp-snapshot-v1" || !Array.isArray(payload.symbols)) {
              throw new Error("compiler response did not contain a workspace symbol snapshot");
            }
            finish(payload.symbols);
          } catch (parseError) {
            appendOutputLine(`Unable to parse EngLang workspace symbols: ${parseError.message}`);
            finish([]);
          }
        }
      );

      cancellationSubscription = cancellationToken?.onCancellationRequested(() => {
        child.kill();
        finish([]);
      });
      if (settled) {
        cancellationSubscription?.dispose?.();
        cancellationSubscription = undefined;
      }
      child.stdin?.on?.("error", (error) => {
        appendOutputLine(`workspace symbol input failed: ${error.message}`);
        child.kill();
        finish([]);
      });
      if (!settled) child.stdin?.end(payload);
    });
  }

  function workspaceOpenDocumentsForFolder(folder) {
    const folderUri = folder.uri.toString();
    return (vscode.workspace.textDocuments ?? []).filter((document) => {
      if (!isEngDocument(document) || document.uri.scheme !== "file") {
        return false;
      }
      const owner = vscode.workspace.getWorkspaceFolder?.(document.uri);
      return owner?.uri?.toString() === folderUri;
    });
  }

  function workspaceDocumentStatesAreCurrent(folder, states) {
    const current = workspaceOpenDocumentsForFolder(folder);
    if (current.length !== states.length) return false;
    const currentByUri = new Map(current.map((document) => [document.uri.toString(), document]));
    return states.every(({ dirty, uri, version }) => {
      const document = currentByUri.get(uri);
      return document?.version === version && document.isDirty === dirty;
    });
  }

  function completionSnapshotForPosition(document, position, context, cancellationToken) {
    return stdinJsonRequest(document, context, cancellationToken, {
      args: [
        "--completion-stdin",
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      errorMessage: "Completion lookup failed",
      parseMessage: "Unable to parse EngLang completion data",
      normalize: (payload) => Array.isArray(payload) ? { completions: payload } : payload
    });
  }

  function definitionSnapshotForPosition(document, position, context, cancellationToken) {
    return stdinJsonRequest(document, context, cancellationToken, {
      args: [
        "--definition-stdin",
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      errorMessage: "Definition lookup failed",
      parseMessage: "Unable to parse EngLang definition data"
    });
  }

  function documentHighlightsForPosition(document, position, context, cancellationToken) {
    return stdinJsonRequest(document, context, cancellationToken, {
      args: [
        "--document-highlights-stdin",
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      errorMessage: "Document highlight lookup failed",
      parseMessage: "Unable to parse EngLang document highlight data",
      normalize: (payload) => Array.isArray(payload) ? payload : []
    });
  }

  function referencesForPosition(
    document,
    position,
    includeDeclaration,
    context,
    cancellationToken
  ) {
    const root = workspaceRoot(document) || nodePath.dirname(document.uri.fsPath);
    return workspaceNavigationJsonRequest(document, context, cancellationToken, {
      args: [
        "--workspace-references-stdin",
        root,
        document.uri.fsPath,
        String(position.line),
        String(position.character),
        includeDeclaration ? "true" : "false"
      ],
      errorMessage: "Reference lookup failed",
      parseMessage: "Unable to parse EngLang reference data",
      root,
      normalize: (payload) => Array.isArray(payload) ? payload : []
    });
  }

  function prepareRenameForPosition(document, position, context, cancellationToken) {
    const root = workspaceRoot(document) || nodePath.dirname(document.uri.fsPath);
    return workspaceNavigationJsonRequest(document, context, cancellationToken, {
      args: [
        "--workspace-prepare-rename-stdin",
        root,
        document.uri.fsPath,
        String(position.line),
        String(position.character)
      ],
      errorMessage: "Rename preparation failed",
      parseMessage: "Unable to parse EngLang rename preparation",
      root
    });
  }

  function renameForPosition(document, position, newName, context, cancellationToken) {
    const root = workspaceRoot(document) || nodePath.dirname(document.uri.fsPath);
    return workspaceNavigationJsonRequest(document, context, cancellationToken, {
      args: [
        "--workspace-rename-stdin",
        root,
        document.uri.fsPath,
        String(position.line),
        String(position.character),
        newName
      ],
      errorMessage: "Rename failed",
      parseMessage: "Unable to parse EngLang rename result",
      root
    });
  }

  function workspaceNavigationJsonRequest(document, context, cancellationToken, request) {
    return new Promise((resolve) => {
      if (!isEngDocument(document) || cancellationToken?.isCancellationRequested) {
        resolve(undefined);
        return;
      }
      const root = request.root;
      const openDocuments = workspaceNavigationDocuments(document, root);
      const documentStates = openDocuments.map((candidate) => ({
        dirty: candidate.isDirty,
        uri: candidate.uri.toString(),
        version: candidate.version
      }));
      const payload = JSON.stringify({
        format: "eng-lsp-open-documents-v1",
        documents: openDocuments.map((candidate) => ({
          path: candidate.uri.fsPath,
          source: candidate.getText()
        }))
      });
      const runtime = findLspRuntime(context, document);
      let settled = false;
      let cancellationSubscription;
      const finish = (value) => {
        if (settled) return;
        settled = true;
        cancellationSubscription?.dispose?.();
        resolve(value);
      };
      const child = cp.execFile(
        runtime,
        request.args,
        { cwd: root, maxBuffer: 10 * 1024 * 1024 },
        (error, stdout, stderr) => {
          if (settled) return;
          if (stderr && stderr.trim().length > 0) appendOutputLine(stderr.trim());
          if (error) {
            appendOutputLine(`${request.errorMessage}: ${error.message}`);
            finish(undefined);
            return;
          }
          if (!workspaceNavigationDocumentStatesAreCurrent(document, root, documentStates)) {
            finish(undefined);
            return;
          }
          try {
            const response = JSON.parse(stdout);
            finish(request.normalize ? request.normalize(response) : response);
          } catch (parseError) {
            appendOutputLine(`${request.parseMessage}: ${parseError.message}`);
            finish(undefined);
          }
        }
      );
      cancellationSubscription = cancellationToken?.onCancellationRequested(() => {
        child.kill();
        finish(undefined);
      });
      if (settled) {
        cancellationSubscription?.dispose?.();
        cancellationSubscription = undefined;
      }
      child.stdin?.on?.("error", (error) => {
        if (settled) return;
        appendOutputLine(`workspace navigation input failed: ${error.message}`);
        child.kill();
        finish(undefined);
      });
      if (!settled) child.stdin?.end(payload);
    });
  }

  function workspaceNavigationDocuments(document, root) {
    const currentUri = document.uri.toString();
    const candidates = [document, ...(vscode.workspace?.textDocuments ?? [])];
    const documents = new Map();
    for (const candidate of candidates) {
      const uri = candidate?.uri;
      if (
        !isEngDocument(candidate)
        || !uri?.fsPath
        || (uri.scheme && uri.scheme !== "file")
        || !pathIsWithinRoot(uri.fsPath, root)
        || (uri.toString() !== currentUri && !candidate.isDirty)
        || typeof candidate.getText !== "function"
      ) {
        continue;
      }
      documents.set(uri.toString(), candidate);
    }
    return Array.from(documents.values());
  }

  function workspaceNavigationDocumentStatesAreCurrent(document, root, states) {
    const current = workspaceNavigationDocuments(document, root);
    if (current.length !== states.length) return false;
    const currentByUri = new Map(current.map((candidate) => [candidate.uri.toString(), candidate]));
    return states.every(({ dirty, uri, version }) => {
      const candidate = currentByUri.get(uri);
      return candidate?.version === version && candidate.isDirty === dirty;
    });
  }

  function pathIsWithinRoot(filePath, root) {
    if (!filePath || !root) return false;
    const relative = nodePath.relative(nodePath.resolve(root), nodePath.resolve(filePath));
    return relative === ""
      || (!nodePath.isAbsolute(relative) && relative !== ".." && !relative.startsWith(`..${nodePath.sep}`));
  }

  function formatDocumentSource(document, context, cancellationToken) {
    return stdinJsonRequest(document, context, cancellationToken, {
      args: ["--format-stdin", document.uri.fsPath],
      errorMessage: "formatting failed",
      parseMessage: "Unable to parse EngLang formatting result"
    });
  }

  function codeActionsForDocumentSource(document, context, cancellationToken) {
    return stdinJsonRequest(document, context, cancellationToken, {
      args: ["--code-actions-stdin", document.uri.fsPath],
      errorMessage: "code action lookup failed",
      parseMessage: "Unable to parse EngLang code actions"
    });
  }

  function stdinJsonRequest(document, context, cancellationToken, request) {
    return new Promise((resolve) => {
      if (!isEngDocument(document) || cancellationToken?.isCancellationRequested) {
        resolve(undefined);
        return;
      }

      const runtime = findLspRuntime(context, document);
      const cwd = workspaceRoot(document);
      const documentVersion = document.version;
      const documentText = document.getText();
      let settled = false;
      let cancellationSubscription;
      const finish = (value) => {
        if (settled) {
          return;
        }
        settled = true;
        cancellationSubscription?.dispose?.();
        if (document.version !== documentVersion) {
          resolve(undefined);
          return;
        }
        resolve(value);
      };

      const child = cp.execFile(
        runtime,
        request.args,
        { cwd, maxBuffer: 10 * 1024 * 1024 },
        (error, stdout, stderr) => {
          if (settled) return;
          if (stderr && stderr.trim().length > 0) {
            appendOutputLine(stderr.trim());
          }
          if (error) {
            appendOutputLine(`${request.errorMessage}: ${error.message}`);
            finish(undefined);
            return;
          }
          try {
            const payload = JSON.parse(stdout);
            finish(request.normalize ? request.normalize(payload) : payload);
          } catch (parseError) {
            appendOutputLine(`${request.parseMessage}: ${parseError.message}`);
            finish(undefined);
          }
        }
      );

      cancellationSubscription = cancellationToken?.onCancellationRequested(() => {
        child.kill();
        finish(undefined);
      });
      if (settled) {
        cancellationSubscription?.dispose?.();
        cancellationSubscription = undefined;
      }

      child.stdin?.on?.("error", (error) => {
        if (settled) return;
        appendOutputLine(`${request.errorMessage} input failed: ${error.message}`);
        child.kill();
        finish(undefined);
      });
      if (!settled) child.stdin?.end(documentText);
    });
  }

  return {
    clearSnapshotCache,
    snapshotDocumentSource,
    workspaceSymbolsForQuery,
    completionSnapshotForPosition,
    definitionSnapshotForPosition,
    documentHighlightsForPosition,
    referencesForPosition,
    prepareRenameForPosition,
    renameForPosition,
    formatDocumentSource,
    codeActionsForDocumentSource
  };
}

module.exports = {
  createLspRequests
};

const cp = require("child_process");
const vscode = require("vscode");

function createLspRequests(options = {}) {
  const isEngDocument = options.isEngDocument ?? (() => true);
  const findLspRuntime = options.findLspRuntime;
  const findLspRuntimeForRoot = options.findLspRuntimeForRoot;
  const workspaceRoot = options.workspaceRoot;
  const appendOutputLine = options.appendOutputLine ?? (() => undefined);
  const snapshotPromiseCache = new Map();

  function clearSnapshotCache(document) {
    snapshotPromiseCache.delete(snapshotCacheKey(document));
  }

  function snapshotDocumentSource(document, context, cancellationToken) {
    const key = snapshotCacheKey(document);
    const cached = snapshotPromiseCache.get(key);
    if (cached) {
      return cached;
    }

    const promise = new Promise((resolve) => {
      const runtime = findLspRuntime(context, document);
      const cwd = workspaceRoot(document);
      const documentVersion = document.version;
      let settled = false;
      const finish = (value) => {
        if (settled) {
          return;
        }
        settled = true;
        if (document.version !== documentVersion) {
          snapshotPromiseCache.delete(key);
          resolve(undefined);
          return;
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

      cancellationToken?.onCancellationRequested(() => {
        child.kill();
        finish(undefined);
      });

      if (child.stdin) {
        child.stdin.end(document.getText());
      }
    });
    snapshotPromiseCache.set(key, promise);
    promise.finally(() => {
      if (snapshotPromiseCache.get(key) === promise) {
        snapshotPromiseCache.delete(key);
      }
    });
    return promise;
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
      let settled = false;
      const finish = (value) => {
        if (settled) {
          return;
        }
        settled = true;
        resolve(value);
      };

      const child = cp.execFile(
        runtime,
        ["--workspace-symbols", root, query ?? ""],
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
          try {
            const payload = JSON.parse(stdout);
            const symbols = Array.isArray(payload)
              ? payload
              : (Array.isArray(payload.symbols) ? payload.symbols : []);
            finish(symbols);
          } catch (parseError) {
            appendOutputLine(`Unable to parse EngLang workspace symbols: ${parseError.message}`);
            finish([]);
          }
        }
      );

      cancellationToken?.onCancellationRequested(() => {
        child.kill();
        finish([]);
      });
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
      if (!isEngDocument(document)) {
        resolve(undefined);
        return;
      }

      const runtime = findLspRuntime(context, document);
      const cwd = workspaceRoot(document);
      let settled = false;
      const finish = (value) => {
        if (settled) {
          return;
        }
        settled = true;
        resolve(value);
      };

      const child = cp.execFile(
        runtime,
        request.args,
        { cwd, maxBuffer: 10 * 1024 * 1024 },
        (error, stdout, stderr) => {
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

      cancellationToken?.onCancellationRequested(() => {
        child.kill();
        finish(undefined);
      });

      if (child.stdin) {
        child.stdin.end(document.getText());
      }
    });
  }

  return {
    clearSnapshotCache,
    snapshotDocumentSource,
    workspaceSymbolsForQuery,
    completionSnapshotForPosition,
    definitionSnapshotForPosition,
    formatDocumentSource,
    codeActionsForDocumentSource
  };
}

module.exports = {
  createLspRequests
};

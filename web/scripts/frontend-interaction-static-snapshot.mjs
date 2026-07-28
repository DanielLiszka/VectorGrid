import http from "node:http";

function mimeType(filePath) {
  if (filePath.endsWith(".html")) return "text/html; charset=utf-8";
  if (filePath.endsWith(".js")) return "text/javascript; charset=utf-8";
  if (filePath.endsWith(".css")) return "text/css; charset=utf-8";
  if (filePath.endsWith(".json")) return "application/json; charset=utf-8";
  if (filePath.endsWith(".svg")) return "image/svg+xml";
  if (filePath.endsWith(".png")) return "image/png";
  if (filePath.endsWith(".ico")) return "image/x-icon";
  return "application/octet-stream";
}

function cloneSnapshotEntries(entries) {
  if (!(entries instanceof Map) || entries.size === 0) {
    throw new Error("served distribution snapshot must be a non-empty Map");
  }
  const cloned = new Map();
  for (const [relativePath, bytes] of entries) {
    if (
      typeof relativePath !== "string" ||
      !relativePath ||
      relativePath.includes("\\") ||
      relativePath.startsWith("/") ||
      relativePath.split("/").some((segment) => {
        return !segment || segment === "." || segment === "..";
      })
    ) {
      throw new Error(`unsafe served snapshot path: ${relativePath}`);
    }
    if (!Buffer.isBuffer(bytes)) {
      throw new Error(`served snapshot entry is not a Buffer: ${relativePath}`);
    }
    cloned.set(relativePath, Buffer.from(bytes));
  }
  if (!cloned.has("index.html")) {
    throw new Error("served distribution snapshot is missing index.html");
  }
  return cloned;
}

export async function startStaticServerFromSnapshot(entries) {
  const servedEntries = cloneSnapshotEntries(entries);
  const server = http.createServer((request, response) => {
    try {
      const requestUrl = new URL(
        String(request.url || "/"),
        "http://127.0.0.1",
      );
      const pathname = decodeURIComponent(requestUrl.pathname);
      const relativePath =
        pathname === "/" ? "index.html" : pathname.replace(/^\/+/, "");
      if (
        relativePath.includes("\\") ||
        relativePath.split("/").some((segment) => {
          return !segment || segment === "." || segment === "..";
        })
      ) {
        response.writeHead(403).end("Forbidden");
        return;
      }
      const bytes = servedEntries.get(relativePath);
      if (!bytes) {
        response.writeHead(404).end("Not found");
        return;
      }
      response.writeHead(200, {
        "Cache-Control": "no-store",
        "Content-Length": bytes.length,
        "Content-Type": mimeType(relativePath),
      });
      response.end(bytes);
    } catch {
      response.writeHead(400).end("Bad request");
    }
  });
  try {
    await new Promise((resolve, reject) => {
      server.once("error", reject);
      server.listen(0, "127.0.0.1", resolve);
    });
    const address = server.address();
    if (!address || typeof address === "string") {
      throw new Error("static server did not expose a TCP address");
    }
    return {
      server,
      url: `http://127.0.0.1:${address.port}/`,
      served_entry_count: servedEntries.size,
      served_total_bytes: [...servedEntries.values()].reduce(
        (total, bytes) => total + bytes.length,
        0,
      ),
    };
  } catch (error) {
    server.closeAllConnections?.();
    if (server.listening) {
      await new Promise((resolve) => server.close(resolve));
    }
    throw error;
  }
}

export async function closeStaticSnapshotServer(server) {
  if (!server) return;
  server.closeAllConnections?.();
  if (!server.listening) return;
  await new Promise((resolve) => server.close(resolve));
}

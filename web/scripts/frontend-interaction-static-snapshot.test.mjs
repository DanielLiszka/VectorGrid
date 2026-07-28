import assert from "node:assert/strict";
import http from "node:http";
import test from "node:test";

import {
  closeStaticSnapshotServer,
  startStaticServerFromSnapshot,
} from "./frontend-interaction-static-snapshot.mjs";

function getBytes(url) {
  return new Promise((resolve, reject) => {
    http
      .get(url, (response) => {
        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () => {
          resolve({
            status: response.statusCode,
            bytes: Buffer.concat(chunks),
          });
        });
        response.on("error", reject);
      })
      .on("error", reject);
  });
}

test("server clones and serves only preloaded snapshot bytes", async () => {
  const original = Buffer.from("<main>attested</main>");
  const entries = new Map([
    ["index.html", original],
    ["assets/app.js", Buffer.from("globalThis.attested = true;")],
  ]);
  const serverInfo = await startStaticServerFromSnapshot(entries);
  try {
    original.fill(0);
    entries.set("index.html", Buffer.from("<main>mutated</main>"));
    entries.delete("assets/app.js");

    const index = await getBytes(serverInfo.url);
    assert.equal(index.status, 200);
    assert.equal(index.bytes.toString("utf8"), "<main>attested</main>");

    const script = await getBytes(`${serverInfo.url}assets/app.js`);
    assert.equal(script.status, 200);
    assert.equal(script.bytes.toString("utf8"), "globalThis.attested = true;");

    const traversal = await getBytes(`${serverInfo.url}%2e%2e/secret.txt`);
    assert.notEqual(traversal.status, 200);
  } finally {
    await closeStaticSnapshotServer(serverInfo.server);
  }
});

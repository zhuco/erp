import { createReadStream, readFileSync, statSync } from "node:fs";
import { createServer } from "node:http";
import { extname, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const HOST = "127.0.0.1";
const PORT = 15440;
const repo = resolve(fileURLToPath(new URL("../../", import.meta.url)));
const ui = resolve(repo, "apps/erp-ui");
const pkg = resolve(ui, "pkg");

const exactFiles = new Map([
  ["/", resolve(ui, "index.html")],
  ["/index.html", resolve(ui, "index.html")],
]);

const mime = new Map([
  [".html", "text/html; charset=utf-8"],
  [".js", "text/javascript; charset=utf-8"],
  [".wasm", "application/wasm"],
]);

const packageFiles = new Map([
  ["/pkg/erp_ui.js", resolve(pkg, "erp_ui.js")],
  ["/pkg/erp_ui_bg.wasm", resolve(pkg, "erp_ui_bg.wasm")],
]);

function requiredFile(path) {
  const stat = statSync(path, { throwIfNoEntry: false });
  if (!stat?.isFile()) throw new Error(`Required Web asset is missing: ${path}`);
}

function validatePackage() {
  const glue = resolve(pkg, "erp_ui.js");
  requiredFile(resolve(ui, "index.html"));
  requiredFile(glue);
  requiredFile(resolve(pkg, "erp_ui_bg.wasm"));

  const source = readFileSync(glue, "utf8");
  const imports = [...source.matchAll(/(?:from\s*|import\s*)["']([^"']+)["']/g)]
    .map((match) => match[1]);
  const expectedSuffixes = new Set([
    "/src/action_store/indexed_db.js",
    "/src/http/download.js",
  ]);
  if (imports.length !== expectedSuffixes.size) {
    throw new Error(`Generated glue has an unexpected module import count: ${imports.length}`);
  }
  for (const specifier of imports) {
    if (!/^\.\/snippets\/[A-Za-z0-9-]+\/src\/(action_store\/indexed_db|http\/download)\.js$/.test(specifier)) {
      throw new Error(`Generated glue imports an unexpected module: ${specifier}`);
    }
    const suffix = specifier.slice(specifier.indexOf("/src/"));
    if (!expectedSuffixes.delete(suffix)) throw new Error(`Duplicate generated adapter: ${suffix}`);
    const path = resolve(pkg, specifier);
    if (!path.startsWith(`${pkg}${sep}`)) throw new Error(`Generated adapter escapes pkg: ${specifier}`);
    requiredFile(path);
    packageFiles.set(`/pkg/${specifier.slice(2).replaceAll("\\", "/")}`, path);
  }
  if (expectedSuffixes.size) throw new Error("Generated glue is missing an expected browser adapter");
}

try {
  validatePackage();
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
  process.exit();
}

const server = createServer((request, response) => {
  response.setHeader("Cache-Control", "no-store");
  response.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  response.setHeader("Cross-Origin-Resource-Policy", "same-origin");
  response.setHeader("X-Content-Type-Options", "nosniff");
  response.setHeader("Referrer-Policy", "no-referrer");

  if (request.method !== "GET" && request.method !== "HEAD") {
    response.writeHead(405, { Allow: "GET, HEAD" }).end("Method not allowed\n");
    return;
  }

  let pathname;
  try {
    pathname = new URL(request.url, `http://${HOST}:${PORT}`).pathname;
  } catch {
    response.writeHead(400).end("Bad request\n");
    return;
  }

  const path = exactFiles.get(pathname) ?? packageFiles.get(pathname);
  const type = path && mime.get(extname(path).toLowerCase());
  const stat = path && type ? statSync(path, { throwIfNoEntry: false }) : null;
  if (!stat?.isFile()) {
    response.writeHead(404).end("Not found\n");
    return;
  }

  response.writeHead(200, {
    "Content-Length": stat.size,
    "Content-Type": type,
  });
  if (request.method === "HEAD") response.end();
  else createReadStream(path).pipe(response);
});

server.on("error", (error) => {
  if (error.code === "EADDRINUSE") {
    console.error(`${HOST}:${PORT} is already in use; the Web acceptance server was not started.`);
  } else {
    console.error(error);
  }
  process.exitCode = 1;
});

server.listen(PORT, HOST, () => {
  console.log(`ERP WASM acceptance UI: http://${HOST}:${PORT}/`);
  console.log(`Static server PID: ${process.pid}`);
  console.log("Expected API origin: http://127.0.0.1:15441");
});

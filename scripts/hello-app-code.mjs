// scripts/hello-app-code.mjs
//
// God-tier runner (WOW harness):
// - Uses ron-app-sdk-ts if we can locate a build artifact exporting `Ron`.
// - Defaults to hitting /site (works with scripts/demo_site.sh).
// - Proves the WOW loop: GET -> POST reload -> GET and verifies x-ron-reload-gen.
// - You can override:
//     RON_BASE_URL=http://127.0.0.1:5304
//     RON_APP_PATH=/site          (default)
//     RON_DO_RELOAD=1             (default: 1)
//     RON_RELOAD_PATH=/reload     (default: "/reload")
//     RON_REPEAT=1                (default: 1)
//
// Usage:
//   RON_BASE_URL=http://127.0.0.1:5304 node scripts/hello-app-code.mjs
//   RON_APP_PATH=/hello RON_BASE_URL=http://127.0.0.1:5304 node scripts/hello-app-code.mjs

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const repoRoot = path.resolve(__dirname, "..");

const baseUrl = process.env.RON_BASE_URL || "http://127.0.0.1:5304";
const appPath = process.env.RON_APP_PATH || "/site";
const doReload = (process.env.RON_DO_RELOAD ?? "1") !== "0";
const reloadSuffix = process.env.RON_RELOAD_PATH || "/reload";
const repeat = Math.max(1, Number(process.env.RON_REPEAT || "1") || 1);

const sdkDistDir = path.join(repoRoot, "sdk", "ron-app-sdk-ts", "dist");

function exists(p) {
  try {
    fs.accessSync(p, fs.constants.R_OK);
    return true;
  } catch {
    return false;
  }
}

function walkFiles(dir, maxDepth) {
  const out = [];
  function walk(d, depth) {
    if (depth > maxDepth) return;
    let ents;
    try {
      ents = fs.readdirSync(d, { withFileTypes: true });
    } catch {
      return;
    }
    for (const e of ents) {
      const p = path.join(d, e.name);
      if (e.isDirectory()) walk(p, depth + 1);
      else out.push(p);
    }
  }
  walk(dir, 0);
  return out;
}

function looksLikeRonExport(jsText) {
  return (
    jsText.includes("class Ron") ||
    jsText.includes("export class Ron") ||
    jsText.includes("exports.Ron") ||
    jsText.includes("Ron =")
  );
}

function pickSdkEntry() {
  const roots = [
    path.join(sdkDistDir, "index.mjs"),
    path.join(sdkDistDir, "index.js"),
    path.join(sdkDistDir, "index.cjs"),
  ];
  for (const p of roots) {
    if (exists(p)) return p;
  }

  if (!exists(sdkDistDir)) return null;

  const candidates = walkFiles(sdkDistDir, 4).filter((p) =>
    /\.(mjs|js|cjs)$/i.test(p),
  );

  for (const p of candidates) {
    let txt = "";
    try {
      const buf = fs.readFileSync(p);
      txt = buf.subarray(0, Math.min(buf.length, 256 * 1024)).toString("utf8");
    } catch {
      continue;
    }
    if (looksLikeRonExport(txt)) return p;
  }

  return null;
}

async function loadRon() {
  const entry = pickSdkEntry();
  if (!entry) {
    throw new Error(
      [
        "ron-app-sdk-ts dist not found.",
        `Expected under: ${sdkDistDir}`,
        "Build it with:",
        "  cd sdk/ron-app-sdk-ts",
        "  pnpm install",
        "  pnpm build",
      ].join("\n"),
    );
  }

  const mod = await import(pathToFileURL(entry).href);
  const Ron = mod?.Ron || mod?.default?.Ron;
  if (!Ron) {
    throw new Error(`SDK loaded from ${entry} but Ron export is missing.`);
  }
  return { Ron, entry };
}

function headerLookup(headersObj, keyLower) {
  if (!headersObj) return null;
  for (const [k, v] of Object.entries(headersObj)) {
    if (k.toLowerCase() === keyLower) return v;
  }
  return null;
}

function parseReloadGen(headersObj) {
  const v = headerLookup(headersObj, "x-ron-reload-gen");
  if (v == null) return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

function printReachabilityHelp(problem) {
  console.error("");
  console.error("Request failed.");
  console.error(`RON_BASE_URL = ${baseUrl}`);
  console.error(`RON_APP_PATH = ${appPath}`);
  console.error("");

  if (problem?.code === "local_network_failure") {
    console.error("Network error: nothing reachable at RON_BASE_URL (or wrong port).");
    console.error("");
    console.error("If you want the app-plane demo gateway, start:");
    console.error("  scripts/demo_site.sh");
    console.error("");
    console.error("Quick probes:");
    console.error("  curl -i http://127.0.0.1:5304/healthz");
    console.error("  curl -i http://127.0.0.1:5304/app/site");
    return;
  }

  console.error("If you got a 404:");
  console.error("  - Most likely you hit the wrong path (ex: /hello instead of /site).");
  console.error("  - For WOW demo site, correct is: /site (=> /app/site on the wire).");
  console.error("");
  console.error("Try:");
  console.error("  RON_BASE_URL=http://127.0.0.1:5304 RON_APP_PATH=/site node scripts/hello-app-code.mjs");
}

async function safeGet(ron, p) {
  try {
    return await ron.get(p);
  } catch (e) {
    return {
      ok: false,
      status: 0,
      problem: { code: "exception", message: String(e?.message || e), kind: "client", retryable: false },
    };
  }
}

async function safePostJson(ron, p, bodyObj) {
  try {
    // Many SDKs provide `post(path, body)`; if yours differs, this will throw and we’ll report cleanly.
    return await ron.post(p, bodyObj);
  } catch (e) {
    return {
      ok: false,
      status: 0,
      problem: { code: "exception", message: String(e?.message || e), kind: "client", retryable: false },
    };
  }
}

async function main() {
  console.log(`[hello-app-code] baseUrl     = ${baseUrl}`);
  console.log(`[hello-app-code] appPath     = ${appPath}`);
  console.log(`[hello-app-code] doReload    = ${doReload ? "1" : "0"}`);
  console.log(`[hello-app-code] reloadSuffix= ${reloadSuffix}`);
  console.log(`[hello-app-code] repeat      = ${repeat}`);
  console.log(`[hello-app-code] sdkDistDir  = ${sdkDistDir}`);

  const { Ron, entry } = await loadRon();
  console.log(`[hello-app-code] SDK entry   = ${entry}`);

  const ron = new Ron({ baseUrl, allowInsecureHttp: true });

  for (let i = 0; i < repeat; i++) {
    if (repeat > 1) console.log(`\n[hello-app-code] --- iteration ${i + 1}/${repeat} ---`);

    // 1) GET
    const a = await safeGet(ron, appPath);
    console.log("[hello-app-code] GET:", JSON.stringify({ ok: a.ok, status: a.status }, null, 2));

    if (!a.ok) {
      console.log(JSON.stringify(a, null, 2));
      printReachabilityHelp(a.problem);
      process.exit(1);
    }

    const genA = parseReloadGen(a.headers);
    console.log(`[hello-app-code] x-ron-reload-gen (before) = ${genA ?? "(missing)"}`);

    // 2) POST reload (optional)
    if (doReload) {
      const reloadPath = `${appPath}${reloadSuffix}`;
      const b = await safePostJson(ron, reloadPath, {});
      console.log("[hello-app-code] POST reload:", JSON.stringify({ ok: b.ok, status: b.status }, null, 2));

      if (!b.ok) {
        console.log(JSON.stringify(b, null, 2));
        console.error("");
        console.error(`[hello-app-code] Reload failed at ${reloadPath}.`);
        console.error("If you're running the site demo, reload endpoint is /site/reload.");
        process.exit(1);
      }

      // 3) GET again and verify header bumped (if present)
      const c = await safeGet(ron, appPath);
      console.log("[hello-app-code] GET(after):", JSON.stringify({ ok: c.ok, status: c.status }, null, 2));

      if (!c.ok) {
        console.log(JSON.stringify(c, null, 2));
        console.error("[hello-app-code] After-reload GET failed.");
        process.exit(1);
      }

      const genC = parseReloadGen(c.headers);
      console.log(`[hello-app-code] x-ron-reload-gen (after)  = ${genC ?? "(missing)"}`);

      if (genA != null && genC != null) {
        if (genC > genA) {
          console.log("[hello-app-code] ✅ WOW: reload generation bumped");
        } else {
          console.log("[hello-app-code] ⚠️  reload generation did NOT bump (header present but unchanged)");
        }
      } else {
        console.log("[hello-app-code] (note) reload-gen header missing; cannot verify bump via headers");
      }
    }
  }

  console.log("\n[hello-app-code] ✅ done");
}

main().catch((e) => {
  console.error("[hello-app-code] FATAL");
  console.error(e?.stack || e);
  process.exit(2);
});

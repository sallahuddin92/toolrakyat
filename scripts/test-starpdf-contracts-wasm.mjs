// Builds isolated Node bindings. Production WASM assets are never regenerated.
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { stdio: "inherit", ...options });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} exited ${result.status}`);
}
const directory = mkdtempSync(join(tmpdir(), "starpdf-contracts-wasm-"));
try {
  run("cargo", ["build", "--manifest-path", "engine/starpdf/Cargo.toml", "--release", "--target", "wasm32-unknown-unknown", "--features", "wasm"]);
  run(process.env.STARPDF_WASM_BINDGEN ?? "wasm-bindgen", ["--target", "nodejs", "--out-dir", directory,
    resolve("engine/starpdf/target/wasm32-unknown-unknown/release/starpdf.wasm")]);
  run("npm", ["exec", "--", "vitest", "run", "src/lib/pdf/execution/contracts.test.ts"], {
    env: { ...process.env, STARPDF_CONTRACTS_WASM: join(directory, "starpdf.js") },
  });
} finally {
  rmSync(directory, { recursive: true, force: true });
}

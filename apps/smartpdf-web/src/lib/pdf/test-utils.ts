import fs from "node:fs";
import path from "node:path";

/**
 * Resolves fixture or asset paths relative to the repository workspace,
 * regardless of whether tests are run from workspace root or inside apps/smartpdf-web.
 */
export function resolveRepoPath(...segments: string[]): string {
  const relativeCandidate = path.join(...segments);
  let curr = process.cwd();
  for (let i = 0; i < 5; i++) {
    const candidate = path.resolve(curr, relativeCandidate);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
    const parent = path.dirname(curr);
    if (parent === curr) break;
    curr = parent;
  }
  return path.resolve(process.cwd(), relativeCandidate);
}

export function loadRepoFixture(...segments: string[]): Uint8Array {
  const resolved = resolveRepoPath(...segments);
  return new Uint8Array(fs.readFileSync(resolved));
}

import { nanoid } from "nanoid";
import { mkdir, rm, readdir, stat } from "node:fs/promises";
import path from "node:path";
import os from "node:os";
import { LIMITS } from "../limits";

// Prefer OS temp dir, but fall back to a repo-local temp directory when the OS
// temp directory is not writable/available (seen in some CI/dev environments).
const TEMP_BASE_DIRS = [
  path.join(os.tmpdir(), "toolrakyat"),
  // Keep fallback outside of the repo so bundlers (Turbopack) don't attempt to
  // treat the project root as a directory asset and traverse it.
  path.join(os.homedir(), ".toolrakyat", "tmp"),
];

async function ensureBaseDirExists() {
  for (const baseDir of TEMP_BASE_DIRS) {
    try {
      await mkdir(baseDir, { recursive: true });
      return baseDir;
    } catch {
      // Try next candidate.
    }
  }

  // Last resort: try the OS temp dir directly (no toolrakyat subfolder) to
  // avoid hard-failing when the subfolder can't be created.
  return os.tmpdir();
}

export async function createTempWorkspace() {
  const baseDir = await ensureBaseDirExists();
  const dir =
    baseDir === os.tmpdir()
      ? path.join(baseDir, `toolrakyat-${nanoid()}`)
      : path.join(baseDir, nanoid());
  await mkdir(dir, { recursive: true });
  
  // Trigger a best-effort cleanup on every workspace creation to keep things tidy
  void cleanupExpiredFiles().catch(() => {});
  
  return dir;
}

export async function cleanupTempWorkspace(dir: string) {
  if (!dir) return;
  const allowed = TEMP_BASE_DIRS.some((baseDir) => dir.startsWith(baseDir));
  const allowedFallback =
    dir.startsWith(os.tmpdir()) && path.basename(dir).startsWith("toolrakyat-");
  if (!allowed && !allowedFallback) return;
  try {
    await rm(dir, { recursive: true, force: true });
  } catch {
    // Best-effort cleanup.
  }
}

/**
 * Removes directories in /tmp/toolrakyat that are older than the max age.
 */
export async function cleanupExpiredFiles(maxAgeMinutes = LIMITS.TEMP_FILE_MAX_AGE_MINUTES) {
  for (const baseDir of TEMP_BASE_DIRS) {
    try {
      const entries = await readdir(baseDir);
      const now = Date.now();
      const maxAgeMs = maxAgeMinutes * 60 * 1000;

      for (const entry of entries) {
        const fullPath = path.join(baseDir, entry);
        const s = await stat(fullPath);

        if (s.isDirectory()) {
          const age = now - s.mtimeMs;
          if (age > maxAgeMs) {
            await rm(fullPath, { recursive: true, force: true });
          }
        }
      }
    } catch {
      // Ignore if baseDir doesn't exist yet or isn't readable.
    }
  }
}

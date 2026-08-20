/**
 * Global and feature limits for ToolRakyat.
 * Configurable via environment variables with safe production defaults.
 */
export const LIMITS = {
  GLOBAL_MAX_FILE_SIZE_MB: Number(process.env.MAX_FILE_SIZE_MB ?? "20"),
  RATE_LIMIT_MAX: Number(process.env.RATE_LIMIT_MAX ?? "20"),
  RATE_LIMIT_BURST: Number(process.env.RATE_LIMIT_BURST ?? "10"),
  TEMP_FILE_MAX_AGE_MINUTES: Number(process.env.TEMP_FILE_MAX_AGE_MINUTES ?? "30"),
} as const;

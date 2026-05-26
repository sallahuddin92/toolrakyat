import { SignJWT, jwtVerify } from "jose";
import { cookies } from "next/headers";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface SessionPayload {
  userId: string;
  tenantId: string;
  businessId: string;
  role: string;
  expiresAt: Date;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const AUTH_SECRET =
  process.env.AUTH_SECRET || "dev-secret-akaunkemas-do-not-use-in-production";

const SESSION_COOKIE_NAME = "ak_session";

/**
 * Encode the secret as a Uint8Array for the `jose` library.
 * The secret must be at least 32 bytes for HS256; we pad/truncate as a
 * pragmatic dev-only fallback. In production, use a strong random 32-byte key.
 */
function getSecretKey(): Uint8Array {
  // DEV-ONLY: when using the hardcoded fallback, log a warning.
  if (!process.env.AUTH_SECRET) {
    console.warn(
      "[DEV-ONLY] AUTH_SECRET env var not set — using insecure hardcoded secret."
    );
  }
  const encoder = new TextEncoder();
  const bytes = encoder.encode(AUTH_SECRET);
  // jose HS256 requires a key of at least 256 bits (32 bytes).
  // If the secret is shorter, we pad with zeros; if longer, we truncate.
  // DEV-ONLY fallback — production must use a proper 32-byte secret.
  const key = new Uint8Array(32);
  key.set(bytes.slice(0, Math.min(bytes.length, 32)));
  return key;
}

const SECRET_KEY = getSecretKey();
const SESSION_DURATION_MS = 7 * 24 * 60 * 60 * 1000; // 7 days

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toTokenPayload(payload: SessionPayload) {
  return {
    sub: payload.userId,
    tid: payload.tenantId,
    bid: payload.businessId,
    role: payload.role,
    exp: Math.floor(payload.expiresAt.getTime() / 1000),
  };
}

function fromTokenPayload(raw: Record<string, unknown>): SessionPayload {
  return {
    userId: String(raw.sub ?? ""),
    tenantId: String(raw.tid ?? ""),
    businessId: String(raw.bid ?? ""),
    role: String(raw.role ?? ""),
    expiresAt: new Date((raw.exp as number) * 1000),
  };
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Encrypt a SessionPayload into a JWT string.
 */
export async function encrypt(payload: SessionPayload): Promise<string> {
  const jwt = await new SignJWT(toTokenPayload(payload) as Record<string, unknown>)
    .setProtectedHeader({ alg: "HS256" })
    .setExpirationTime(Math.floor(payload.expiresAt.getTime() / 1000))
    .setIssuedAt()
    .sign(SECRET_KEY);
  return jwt;
}

/**
 * Decrypt (verify) a JWT string back into a SessionPayload, or null if invalid.
 */
export async function decrypt(session: string): Promise<SessionPayload | null> {
  try {
    const { payload } = await jwtVerify(session, SECRET_KEY, {
      algorithms: ["HS256"],
    });
    return fromTokenPayload(payload);
  } catch {
    return null;
  }
}

/**
 * Create an encrypted session cookie.
 *
 * DEV-ONLY: No database — the session payload is encoded directly into the JWT.
 */
export async function createSession(
  userId: string,
  tenantId: string,
  businessId: string,
  role: string
): Promise<string> {
  // DEV-ONLY: In-memory session payload — no database involved.
  const expiresAt = new Date(Date.now() + SESSION_DURATION_MS);
  const payload: SessionPayload = { userId, tenantId, businessId, role, expiresAt };
  const session = await encrypt(payload);

  const cookieStore = await cookies();
  cookieStore.set(SESSION_COOKIE_NAME, session, {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    expires: expiresAt,
  });

  return session;
}

/**
 * Delete the session cookie (sign out).
 */
export async function deleteSession(): Promise<void> {
  const cookieStore = await cookies();
  cookieStore.set(SESSION_COOKIE_NAME, "", {
    httpOnly: true,
    secure: process.env.NODE_ENV === "production",
    sameSite: "lax",
    path: "/",
    expires: new Date(0),
  });
}

/**
 * Read the session cookie value from the request (server-only).
 * Returns the raw JWT string or null if no cookie exists.
 */
export async function getSessionCookie(): Promise<string | null> {
  const cookieStore = await cookies();
  const cookie = cookieStore.get(SESSION_COOKIE_NAME);
  return cookie?.value ?? null;
}

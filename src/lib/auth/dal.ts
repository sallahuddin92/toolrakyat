import { cache } from "react";
import { decrypt, getSessionCookie, type SessionPayload } from "./session";
import { db, schema } from "@/lib/db";
import { eq } from "drizzle-orm";

// ---------------------------------------------------------------------------
// Demo mode
// ---------------------------------------------------------------------------

/**
 * DEV-ONLY: Returns a hardcoded demo session payload when demo mode is active.
 */
function makeDemoSession(): SessionPayload {
  console.warn(
    "[DEV-ONLY] NEXT_PUBLIC_DEMO_MODE is enabled — returning demo user session. " +
    "No real authentication is in place."
  );
  return {
    userId: "demo-user-001",
    tenantId: "demo-tenant-001",
    businessId: "demo-business-001",
    role: "admin",
    expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
  };
}

// ---------------------------------------------------------------------------
// Server-side session verification
// ---------------------------------------------------------------------------

/**
 * Verify the current session.
 *
 * Cached per-request via React's `cache()` to avoid decrypting the JWT twice
 * during a single server render.
 */
export const verifySession = cache(
  async (): Promise<SessionPayload | null> => {
    // DEV-ONLY: Skip real auth when demo mode is active.
    if (process.env.NEXT_PUBLIC_DEMO_MODE === "true") {
      return makeDemoSession();
    }

    const cookieValue = await getSessionCookie();
    if (!cookieValue) {
      return null;
    }

    const session = await decrypt(cookieValue);
    if (!session) {
      return null;
    }

    // Check expiry
    if (new Date() > session.expiresAt) {
      return null;
    }

    return session;
  }
);

/**
 * Get the current user. Throws if no valid session exists.
 */
export async function getCurrentUser(): Promise<SessionPayload> {
  const session = await verifySession();
  if (!session) {
    throw new Error("Unauthorized: no valid session found.");
  }
  return session;
}

/**
 * Require the current user to have one of the specified roles.
 * Throws if no session or if the user's role does not match.
 */
export async function requireRole(...roles: string[]): Promise<SessionPayload> {
  const session = await getCurrentUser();
  if (!roles.includes(session.role)) {
    throw new Error(
      `Forbidden: role "${session.role}" is not in allowed roles [${roles.join(", ")}].`
    );
  }
  return session;
}

// ---------------------------------------------------------------------------
// User profile lookup
// ---------------------------------------------------------------------------

/**
 * Get a user's profile from the database.
 *
 * Falls back to a demo stub when NEXT_PUBLIC_DEMO_MODE is enabled.
 */
export async function getUserProfile(userId: string) {
  if (process.env.NEXT_PUBLIC_DEMO_MODE === "true") {
    return { id: userId, email: "demo@akaunkemas.my", name: "Demo User" };
  }

  const user = db
    .select({
      id: schema.users.id,
      email: schema.users.email,
      name: schema.users.name,
    })
    .from(schema.users)
    .where(eq(schema.users.id, userId))
    .get();

  return user ?? null;
}

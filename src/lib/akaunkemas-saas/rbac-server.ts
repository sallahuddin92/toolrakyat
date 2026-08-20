import { getCurrentUser, verifySession } from "@/lib/auth/dal";
import { redirect } from "next/navigation";
import {
  type Role,
  type TenantContext,
  hasRole,
  canManageBusiness,
  canEditTransactions,
  canEditReceipts,
  canManageMatches,
  canGenerateAccountantPack,
  canViewAuditLogs,
  canManageMembers,
} from "./rbac";

// Re-export everything from rbac.ts so consumers only need one import
export * from "./rbac";

// ---------------------------------------------------------------------------
// Server-side session & tenant context
// ---------------------------------------------------------------------------

/**
 * Get the current tenant context from the session.
 * Throws if no valid session.
 */
export async function getTenantContext(): Promise<TenantContext> {
  const session = await getCurrentUser();
  return {
    tenantId: session.tenantId,
    businessId: session.businessId,
    userId: session.userId,
    role: session.role as Role,
  };
}

// ---------------------------------------------------------------------------
// Permission enforcement helpers
// ---------------------------------------------------------------------------

/**
 * Require the current user to have at least the given role.
 * Returns the TenantContext if authorized, throws if not.
 */
export async function requireRole(requiredRole: Role): Promise<TenantContext> {
  const ctx = await getTenantContext();
  if (!hasRole(ctx.role, requiredRole)) {
    throw new Error(
      `Forbidden: role "${ctx.role}" is not sufficient. Required: ${requiredRole}`,
    );
  }
  return ctx;
}

/**
 * Require the user to pass a custom permission check.
 * Returns context if authorized, throws if not.
 */
export async function requirePermission(
  check: (role: Role) => boolean,
  errorMessage = "Forbidden: insufficient permissions.",
): Promise<TenantContext> {
  const ctx = await getTenantContext();
  if (!check(ctx.role)) {
    throw new Error(errorMessage);
  }
  return ctx;
}

// ---------------------------------------------------------------------------
// Convenience wrappers: combine session + permission check
// ---------------------------------------------------------------------------

/** Require permission to edit transactions (OWNER, ADMIN, STAFF). */
export async function requireEditTransactions(): Promise<TenantContext> {
  return requirePermission(
    canEditTransactions,
    "Forbidden: cannot edit transactions.",
  );
}

/** Require permission to edit receipts (OWNER, ADMIN, STAFF). */
export async function requireEditReceipts(): Promise<TenantContext> {
  return requirePermission(canEditReceipts, "Forbidden: cannot edit receipts.");
}

/** Require permission to manage matches (OWNER, ADMIN, STAFF). */
export async function requireManageMatches(): Promise<TenantContext> {
  return requirePermission(
    canManageMatches,
    "Forbidden: cannot manage matches.",
  );
}

/** Require permission to generate accountant packs (OWNER, ADMIN, ACCOUNTANT). */
export async function requireGeneratePack(): Promise<TenantContext> {
  return requirePermission(
    canGenerateAccountantPack,
    "Forbidden: cannot generate accountant packs.",
  );
}

/** Require permission to view audit logs (OWNER, ADMIN). */
export async function requireViewAuditLogs(): Promise<TenantContext> {
  return requirePermission(
    canViewAuditLogs,
    "Forbidden: cannot view audit logs.",
  );
}

/** Require permission to manage members (OWNER, ADMIN). */
export async function requireManageMembers(): Promise<TenantContext> {
  return requirePermission(
    canManageMembers,
    "Forbidden: cannot manage members.",
  );
}

/** Require permission to manage business (OWNER, ADMIN). */
export async function requireManageBusiness(): Promise<TenantContext> {
  return requirePermission(
    canManageBusiness,
    "Forbidden: cannot manage business.",
  );
}

// ---------------------------------------------------------------------------
// Route protection for server components / layouts
// ---------------------------------------------------------------------------

/**
 * Protect a server component or layout.
 * Redirects to login if no valid session.
 * Returns the tenant context if authenticated.
 *
 * Call this at the top of server components that need auth.
 */
export async function protectRoute(): Promise<TenantContext> {
  const session = await verifySession();
  if (!session) {
    redirect("/app/akaunkemas/login");
  }
  return {
    tenantId: session.tenantId,
    businessId: session.businessId,
    userId: session.userId,
    role: session.role as Role,
  };
}

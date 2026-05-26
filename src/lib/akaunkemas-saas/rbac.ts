import { z } from "zod";

// ---------------------------------------------------------------------------
// Role constants & type
// ---------------------------------------------------------------------------

export const ROLES = {
  OWNER: "owner",
  ADMIN: "admin",
  STAFF: "staff",
  ACCOUNTANT: "accountant",
  VIEWER: "viewer",
} as const;

export type Role = (typeof ROLES)[keyof typeof ROLES];

// ---------------------------------------------------------------------------
// Role hierarchy (ordered from highest to lowest privilege)
// OWNER > ADMIN > STAFF > ACCOUNTANT > VIEWER
// ---------------------------------------------------------------------------

const ROLE_RANK: Record<Role, number> = {
  [ROLES.OWNER]: 5,
  [ROLES.ADMIN]: 4,
  [ROLES.STAFF]: 3,
  [ROLES.ACCOUNTANT]: 2,
  [ROLES.VIEWER]: 1,
};

/**
 * Returns true if `userRole` is at least as privileged as `requiredRole`.
 */
export function hasRole(userRole: Role, requiredRole: Role): boolean {
  return ROLE_RANK[userRole] >= ROLE_RANK[requiredRole];
}

// ---------------------------------------------------------------------------
// Permission checkers
// ---------------------------------------------------------------------------

/** OWNER, ADMIN */
export function canManageBusiness(role: Role): boolean {
  return hasRole(role, ROLES.ADMIN);
}

/** OWNER, ADMIN, STAFF */
export function canEditTransactions(role: Role): boolean {
  return hasRole(role, ROLES.STAFF);
}

/** All roles including VIEWER */
export function canViewTransactions(_role: Role): boolean {
  return true;
}

/** OWNER, ADMIN, STAFF */
export function canEditReceipts(role: Role): boolean {
  return hasRole(role, ROLES.STAFF);
}

/** OWNER, ADMIN, STAFF */
export function canManageMatches(role: Role): boolean {
  return hasRole(role, ROLES.STAFF);
}

/** OWNER, ADMIN, ACCOUNTANT */
export function canGenerateAccountantPack(role: Role): boolean {
  return (
    hasRole(role, ROLES.ADMIN) || role === ROLES.ACCOUNTANT
  );
}

/** OWNER, ADMIN */
export function canViewAuditLogs(role: Role): boolean {
  return hasRole(role, ROLES.ADMIN);
}

/** OWNER, ADMIN */
export function canManageMembers(role: Role): boolean {
  return hasRole(role, ROLES.ADMIN);
}

/** OWNER, ADMIN */
export function canViewSettings(role: Role): boolean {
  return hasRole(role, ROLES.ADMIN);
}

// ---------------------------------------------------------------------------
// Tenant / Business isolation
// ---------------------------------------------------------------------------

export interface TenantContext {
  tenantId: string;
  businessId: string;
  userId: string;
  role: Role;
}

/**
 * Returns true when the context tenantId matches the target.
 */
export function isAuthorizedForTenant(
  ctx: TenantContext,
  targetTenantId: string,
): boolean {
  return ctx.tenantId === targetTenantId;
}

/**
 * Returns true when the context businessId matches the target.
 */
export function isAuthorizedForBusiness(
  ctx: TenantContext,
  targetBusinessId: string,
): boolean {
  return ctx.businessId === targetBusinessId;
}

// ---------------------------------------------------------------------------
// Zod schema for TenantContext
// ---------------------------------------------------------------------------

export const TenantContextSchema = z.object({
  tenantId: z.string().min(1),
  businessId: z.string().min(1),
  userId: z.string().min(1),
  role: z.enum(["owner", "admin", "staff", "accountant", "viewer"]),
});

// ---------------------------------------------------------------------------
// Tenant-scoped filter helper
// ---------------------------------------------------------------------------

/**
 * Returns a filter object that can be spread into a query to enforce
 * tenant+business isolation.
 */
export function tenantFilter(
  ctx: TenantContext,
): { tenantId: string; businessId: string } {
  return { tenantId: ctx.tenantId, businessId: ctx.businessId };
}

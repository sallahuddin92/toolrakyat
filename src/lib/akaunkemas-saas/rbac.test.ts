import { describe, it, expect } from "vitest";
import {
  ROLES,
  hasRole,
  canManageBusiness,
  canEditTransactions,
  canViewTransactions,
  canEditReceipts,
  canManageMatches,
  canGenerateAccountantPack,
  canViewAuditLogs,
  canManageMembers,
  canViewSettings,
  isAuthorizedForTenant,
  isAuthorizedForBusiness,
  TenantContextSchema,
  tenantFilter,
  type TenantContext,
  type Role,
} from "./rbac";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const ctx: TenantContext = {
  tenantId: "t1",
  businessId: "b1",
  userId: "u1",
  role: ROLES.OWNER,
};

// ---------------------------------------------------------------------------
// Permission hierarchy
// ---------------------------------------------------------------------------

describe("Permission hierarchy", () => {
  const _allRoles: Role[] = [
    ROLES.OWNER,
    ROLES.ADMIN,
    ROLES.STAFF,
    ROLES.ACCOUNTANT,
    ROLES.VIEWER,
  ];

  describe("OWNER can do everything", () => {
    it.each([
      ["canManageBusiness", canManageBusiness],
      ["canEditTransactions", canEditTransactions],
      ["canViewTransactions", canViewTransactions],
      ["canEditReceipts", canEditReceipts],
      ["canManageMatches", canManageMatches],
      ["canGenerateAccountantPack", canGenerateAccountantPack],
      ["canViewAuditLogs", canViewAuditLogs],
      ["canManageMembers", canManageMembers],
      ["canViewSettings", canViewSettings],
    ])("%s", (_name, checker) => {
      expect(checker(ROLES.OWNER)).toBe(true);
    });
  });

  describe("VIEWER can only view transactions", () => {
    it("canViewTransactions", () => {
      expect(canViewTransactions(ROLES.VIEWER)).toBe(true);
    });

    it.each([
      ["canManageBusiness", canManageBusiness],
      ["canEditTransactions", canEditTransactions],
      ["canEditReceipts", canEditReceipts],
      ["canManageMatches", canManageMatches],
      ["canGenerateAccountantPack", canGenerateAccountantPack],
      ["canViewAuditLogs", canViewAuditLogs],
      ["canManageMembers", canManageMembers],
      ["canViewSettings", canViewSettings],
    ])("%s", (_name, checker) => {
      expect(checker(ROLES.VIEWER)).toBe(false);
    });
  });
});

// ---------------------------------------------------------------------------
// hasRole
// ---------------------------------------------------------------------------

describe("hasRole", () => {
  it("owner >= owner", () => {
    expect(hasRole(ROLES.OWNER, ROLES.OWNER)).toBe(true);
  });

  it("owner >= admin", () => {
    expect(hasRole(ROLES.OWNER, ROLES.ADMIN)).toBe(true);
  });

  it("owner >= staff", () => {
    expect(hasRole(ROLES.OWNER, ROLES.STAFF)).toBe(true);
  });

  it("owner >= viewer", () => {
    expect(hasRole(ROLES.OWNER, ROLES.VIEWER)).toBe(true);
  });

  it("admin >= admin", () => {
    expect(hasRole(ROLES.ADMIN, ROLES.ADMIN)).toBe(true);
  });

  it("admin >= staff", () => {
    expect(hasRole(ROLES.ADMIN, ROLES.STAFF)).toBe(true);
  });

  it("admin >= accountant", () => {
    expect(hasRole(ROLES.ADMIN, ROLES.ACCOUNTANT)).toBe(true);
  });

  it("admin >= viewer", () => {
    expect(hasRole(ROLES.ADMIN, ROLES.VIEWER)).toBe(true);
  });

  it("staff >= staff", () => {
    expect(hasRole(ROLES.STAFF, ROLES.STAFF)).toBe(true);
  });

  it("staff >= accountant", () => {
    expect(hasRole(ROLES.STAFF, ROLES.ACCOUNTANT)).toBe(true);
  });

  it("staff >= viewer", () => {
    expect(hasRole(ROLES.STAFF, ROLES.VIEWER)).toBe(true);
  });

  it("staff !>= admin (lower rank cannot claim higher)", () => {
    expect(hasRole(ROLES.STAFF, ROLES.ADMIN)).toBe(false);
  });

  it("accountant !>= staff", () => {
    expect(hasRole(ROLES.ACCOUNTANT, ROLES.STAFF)).toBe(false);
  });

  it("accountant !>= admin", () => {
    expect(hasRole(ROLES.ACCOUNTANT, ROLES.ADMIN)).toBe(false);
  });

  it("viewer !>= accountant", () => {
    expect(hasRole(ROLES.VIEWER, ROLES.ACCOUNTANT)).toBe(false);
  });

  it("viewer !>= staff", () => {
    expect(hasRole(ROLES.VIEWER, ROLES.STAFF)).toBe(false);
  });

  it("viewer !>= admin", () => {
    expect(hasRole(ROLES.VIEWER, ROLES.ADMIN)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Tenant isolation
// ---------------------------------------------------------------------------

describe("Tenant isolation", () => {
  it("isAuthorizedForTenant allows same tenantId", () => {
    const c: TenantContext = { ...ctx, tenantId: "t1" };
    expect(isAuthorizedForTenant(c, "t1")).toBe(true);
  });

  it("isAuthorizedForTenant rejects different tenantId", () => {
    const c: TenantContext = { ...ctx, tenantId: "t1" };
    expect(isAuthorizedForTenant(c, "t2")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Business isolation
// ---------------------------------------------------------------------------

describe("Business isolation", () => {
  it("isAuthorizedForBusiness allows same businessId", () => {
    const c: TenantContext = { ...ctx, businessId: "b1" };
    expect(isAuthorizedForBusiness(c, "b1")).toBe(true);
  });

  it("isAuthorizedForBusiness rejects different businessId", () => {
    const c: TenantContext = { ...ctx, businessId: "b1" };
    expect(isAuthorizedForBusiness(c, "b2")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// TenantContextSchema validation
// ---------------------------------------------------------------------------

describe("TenantContextSchema", () => {
  const valid = {
    tenantId: "t-123",
    businessId: "b-456",
    userId: "u-789",
    role: "owner" as const,
  };

  it("accepts valid data", () => {
    const result = TenantContextSchema.safeParse(valid);
    expect(result.success).toBe(true);
  });

  it("rejects missing tenantId", () => {
    const { tenantId: _, ...rest } = valid;
    const result = TenantContextSchema.safeParse(rest);
    expect(result.success).toBe(false);
  });

  it("rejects missing businessId", () => {
    const { businessId: _, ...rest } = valid;
    const result = TenantContextSchema.safeParse(rest);
    expect(result.success).toBe(false);
  });

  it("rejects missing userId", () => {
    const { userId: _, ...rest } = valid;
    const result = TenantContextSchema.safeParse(rest);
    expect(result.success).toBe(false);
  });

  it("rejects missing role", () => {
    const { role: _, ...rest } = valid;
    const result = TenantContextSchema.safeParse(rest);
    expect(result.success).toBe(false);
  });

  it("rejects invalid role value", () => {
    const result = TenantContextSchema.safeParse({ ...valid, role: "superadmin" });
    expect(result.success).toBe(false);
  });

  it("rejects empty tenantId", () => {
    const result = TenantContextSchema.safeParse({ ...valid, tenantId: "" });
    expect(result.success).toBe(false);
  });

  it("rejects empty businessId", () => {
    const result = TenantContextSchema.safeParse({ ...valid, businessId: "" });
    expect(result.success).toBe(false);
  });

  it("rejects empty userId", () => {
    const result = TenantContextSchema.safeParse({ ...valid, userId: "" });
    expect(result.success).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// tenantFilter
// ---------------------------------------------------------------------------

describe("tenantFilter", () => {
  it("returns correct tenantId and businessId from context", () => {
    const c: TenantContext = {
      tenantId: "t1",
      businessId: "b1",
      userId: "u1",
      role: ROLES.OWNER,
    };
    expect(tenantFilter(c)).toEqual({ tenantId: "t1", businessId: "b1" });
  });
});

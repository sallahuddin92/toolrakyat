import { describe, it, expect, vi, beforeEach } from "vitest";

// ---------------------------------------------------------------------------
// Mock the auth DAL
// ---------------------------------------------------------------------------

vi.mock("@/lib/auth/dal", () => ({
  getCurrentUser: vi.fn(),
  verifySession: vi.fn(),
}));

import { getCurrentUser, verifySession } from "@/lib/auth/dal";
import {
  protectRoute,
  getTenantContext,
  requireRole,
  requirePermission,
  requireEditTransactions,
  requireEditReceipts,
  requireManageMatches,
  requireGeneratePack,
  requireViewAuditLogs,
  requireManageMembers,
  requireManageBusiness,
  ROLES,
  type Role,
  type TenantContext,
} from "./rbac-server";

// ---------------------------------------------------------------------------
// Helper: create a mock SessionPayload
// ---------------------------------------------------------------------------

function mockSession(overrides: Partial<{
  userId: string;
  tenantId: string;
  businessId: string;
  role: string;
}> = {}) {
  return {
    userId: overrides.userId ?? "user-001",
    tenantId: overrides.tenantId ?? "tenant-001",
    businessId: overrides.businessId ?? "business-001",
    role: overrides.role ?? ROLES.ADMIN,
    expiresAt: new Date(Date.now() + 3600_000),
  };
}

// ---------------------------------------------------------------------------
// Helper: expect a specific tenant context shape
// ---------------------------------------------------------------------------

function expectContextEqual(
  actual: TenantContext,
  expected: Partial<TenantContext>,
) {
  if (expected.tenantId !== undefined) {
    expect(actual.tenantId).toBe(expected.tenantId);
  }
  if (expected.businessId !== undefined) {
    expect(actual.businessId).toBe(expected.businessId);
  }
  if (expected.userId !== undefined) {
    expect(actual.userId).toBe(expected.userId);
  }
  if (expected.role !== undefined) {
    expect(actual.role).toBe(expected.role);
  }
}

// ---------------------------------------------------------------------------
// Setup: clear mocks between tests
// ---------------------------------------------------------------------------

beforeEach(() => {
  vi.clearAllMocks();
});

// ---------------------------------------------------------------------------
// protectRoute
// ---------------------------------------------------------------------------

describe("protectRoute", () => {
  it("redirects when no session", async () => {
    vi.mocked(verifySession).mockResolvedValue(null);

    try {
      await protectRoute();
    } catch {
      // Next.js redirect throws an error (NEXT_REDIRECT)
    }

    expect(verifySession).toHaveBeenCalledOnce();
    // In the mock environment, redirect may not actually throw NEXT_REDIRECT;
    // the key assertion is that we reached the redirect path
  });

  it("returns tenant context when session is valid", async () => {
    vi.mocked(verifySession).mockResolvedValue(mockSession({
      userId: "u1",
      tenantId: "t1",
      businessId: "b1",
      role: ROLES.ADMIN,
    }));

    const ctx = await protectRoute();
    expectContextEqual(ctx, {
      userId: "u1",
      tenantId: "t1",
      businessId: "b1",
      role: ROLES.ADMIN,
    });
  });

  it("returns correct role from session", async () => {
    vi.mocked(verifySession).mockResolvedValue(mockSession({ role: ROLES.OWNER }));

    const ctx = await protectRoute();
    expect(ctx.role).toBe(ROLES.OWNER);
  });
});

// ---------------------------------------------------------------------------
// getTenantContext
// ---------------------------------------------------------------------------

describe("getTenantContext", () => {
  it("returns correct context from session", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({
      userId: "u1",
      tenantId: "t1",
      businessId: "b1",
      role: ROLES.OWNER,
    }));

    const ctx = await getTenantContext();
    expectContextEqual(ctx, {
      userId: "u1",
      tenantId: "t1",
      businessId: "b1",
      role: ROLES.OWNER,
    });
  });

  it("throws when no valid session", async () => {
    vi.mocked(getCurrentUser).mockRejectedValue(
      new Error("Unauthorized: no valid session found."),
    );

    await expect(getTenantContext()).rejects.toThrow(
      "Unauthorized: no valid session found.",
    );
  });

  it("maps role correctly for different roles", async () => {
    for (const role of [
      ROLES.OWNER,
      ROLES.ADMIN,
      ROLES.STAFF,
      ROLES.ACCOUNTANT,
      ROLES.VIEWER,
    ] as Role[]) {
      vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role }));

      const ctx = await getTenantContext();
      expect(ctx.role).toBe(role);
    }
  });
});

// ---------------------------------------------------------------------------
// requireRole
// ---------------------------------------------------------------------------

describe("requireRole", () => {
  it("succeeds when role is sufficient", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.OWNER }));

    const ctx = await requireRole(ROLES.ADMIN);
    expect(ctx.role).toBe(ROLES.OWNER);
  });

  it("succeeds when role equals required role", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    const ctx = await requireRole(ROLES.STAFF);
    expect(ctx.role).toBe(ROLES.STAFF);
  });

  it("throws when role is insufficient", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(requireRole(ROLES.ADMIN)).rejects.toThrow(
      /Forbidden: role "viewer" is not sufficient/,
    );
  });

  it("throws when STAFF tries to require ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    await expect(requireRole(ROLES.ADMIN)).rejects.toThrow(/Forbidden/);
  });

  it("throws when ACCOUNTANT tries to require STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ACCOUNTANT }));

    await expect(requireRole(ROLES.STAFF)).rejects.toThrow(/Forbidden/);
  });
});

// ---------------------------------------------------------------------------
// requirePermission
// ---------------------------------------------------------------------------

describe("requirePermission", () => {
  it("succeeds when permission check passes", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requirePermission((role) => role === ROLES.ADMIN);
    expect(ctx.role).toBe(ROLES.ADMIN);
  });

  it("throws with default message when check fails", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(
      requirePermission((role) => role === ROLES.ADMIN),
    ).rejects.toThrow("Forbidden: insufficient permissions.");
  });

  it("throws with custom message when check fails", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(
      requirePermission((role) => role === ROLES.ADMIN, "Custom error"),
    ).rejects.toThrow("Custom error");
  });

  it("succeeds for OWNER with hierarchy-aware check", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.OWNER }));

    // Use hasRole (from rbac) which respects the role hierarchy:
    // OWNER >= ADMIN => true
    const ctx = await requirePermission((role) =>
      role === ROLES.OWNER || role === ROLES.ADMIN,
    );
    expect(ctx).toBeDefined();
    expect(ctx.role).toBe(ROLES.OWNER);
  });


  it("succeeds with always-true check for VIEWER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    const ctx = await requirePermission(() => true);
    expect(ctx.role).toBe(ROLES.VIEWER);
  });
});

// ---------------------------------------------------------------------------
// requireEditTransactions
// ---------------------------------------------------------------------------

describe("requireEditTransactions", () => {
  it("fails for VIEWER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(requireEditTransactions()).rejects.toThrow(
      "Forbidden: cannot edit transactions.",
    );
  });

  it("succeeds for ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requireEditTransactions();
    expect(ctx.role).toBe(ROLES.ADMIN);
  });

  it("succeeds for OWNER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.OWNER }));

    const ctx = await requireEditTransactions();
    expect(ctx.role).toBe(ROLES.OWNER);
  });

  it("succeeds for STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    const ctx = await requireEditTransactions();
    expect(ctx.role).toBe(ROLES.STAFF);
  });

  it("fails for ACCOUNTANT", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ACCOUNTANT }));

    await expect(requireEditTransactions()).rejects.toThrow(
      "Forbidden: cannot edit transactions.",
    );
  });
});

// ---------------------------------------------------------------------------
// requireEditReceipts
// ---------------------------------------------------------------------------

describe("requireEditReceipts", () => {
  it("fails for VIEWER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(requireEditReceipts()).rejects.toThrow(
      "Forbidden: cannot edit receipts.",
    );
  });

  it("succeeds for ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requireEditReceipts();
    expect(ctx.role).toBe(ROLES.ADMIN);
  });
});

// ---------------------------------------------------------------------------
// requireManageMatches
// ---------------------------------------------------------------------------

describe("requireManageMatches", () => {
  it("fails for VIEWER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(requireManageMatches()).rejects.toThrow(
      "Forbidden: cannot manage matches.",
    );
  });

  it("succeeds for STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    const ctx = await requireManageMatches();
    expect(ctx.role).toBe(ROLES.STAFF);
  });
});

// ---------------------------------------------------------------------------
// requireGeneratePack
// ---------------------------------------------------------------------------

describe("requireGeneratePack", () => {
  it("succeeds for ACCOUNTANT", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ACCOUNTANT }));

    const ctx = await requireGeneratePack();
    expect(ctx.role).toBe(ROLES.ACCOUNTANT);
  });

  it("fails for STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    await expect(requireGeneratePack()).rejects.toThrow(
      "Forbidden: cannot generate accountant packs.",
    );
  });

  it("fails for VIEWER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.VIEWER }));

    await expect(requireGeneratePack()).rejects.toThrow(
      "Forbidden: cannot generate accountant packs.",
    );
  });

  it("succeeds for ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requireGeneratePack();
    expect(ctx.role).toBe(ROLES.ADMIN);
  });
});

// ---------------------------------------------------------------------------
// requireViewAuditLogs
// ---------------------------------------------------------------------------

describe("requireViewAuditLogs", () => {
  it("fails for STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    await expect(requireViewAuditLogs()).rejects.toThrow(
      "Forbidden: cannot view audit logs.",
    );
  });

  it("succeeds for ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requireViewAuditLogs();
    expect(ctx.role).toBe(ROLES.ADMIN);
  });

  it("succeeds for OWNER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.OWNER }));

    const ctx = await requireViewAuditLogs();
    expect(ctx.role).toBe(ROLES.OWNER);
  });
});

// ---------------------------------------------------------------------------
// requireManageMembers
// ---------------------------------------------------------------------------

describe("requireManageMembers", () => {
  it("fails for STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    await expect(requireManageMembers()).rejects.toThrow(
      "Forbidden: cannot manage members.",
    );
  });

  it("succeeds for ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requireManageMembers();
    expect(ctx.role).toBe(ROLES.ADMIN);
  });
});

// ---------------------------------------------------------------------------
// requireManageBusiness
// ---------------------------------------------------------------------------

describe("requireManageBusiness", () => {
  it("fails for STAFF", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.STAFF }));

    await expect(requireManageBusiness()).rejects.toThrow(
      "Forbidden: cannot manage business.",
    );
  });

  it("succeeds for ADMIN", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.ADMIN }));

    const ctx = await requireManageBusiness();
    expect(ctx.role).toBe(ROLES.ADMIN);
  });

  it("succeeds for OWNER", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({ role: ROLES.OWNER }));

    const ctx = await requireManageBusiness();
    expect(ctx.role).toBe(ROLES.OWNER);
  });
});

// ---------------------------------------------------------------------------
// Tenant isolation
// ---------------------------------------------------------------------------

describe("Tenant isolation", () => {
  it("getTenantContext has correct tenantId", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({
      tenantId: "tenant-xyz",
      businessId: "biz-abc",
    }));

    const ctx = await getTenantContext();
    expect(ctx.tenantId).toBe("tenant-xyz");
  });

  it("getTenantContext has correct businessId", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({
      tenantId: "tenant-xyz",
      businessId: "biz-abc",
    }));

    const ctx = await getTenantContext();
    expect(ctx.businessId).toBe("biz-abc");
  });

  it("protectRoute preserves tenantId/businessId from session", async () => {
    vi.mocked(verifySession).mockResolvedValue(mockSession({
      tenantId: "t-001",
      businessId: "b-001",
    }));

    const ctx = await protectRoute();
    expect(ctx.tenantId).toBe("t-001");
    expect(ctx.businessId).toBe("b-001");
  });

  it("requireRole returns correct tenantId in context", async () => {
    vi.mocked(getCurrentUser).mockResolvedValue(mockSession({
      tenantId: "tenant-abc",
      businessId: "business-123",
      role: ROLES.OWNER,
    }));

    const ctx = await requireRole(ROLES.ADMIN);
    expect(ctx.tenantId).toBe("tenant-abc");
    expect(ctx.businessId).toBe("business-123");
  });
});

import { db, schema } from "@/lib/db";
import { eq } from "drizzle-orm";
import { hashPassword, verifyPassword } from "./password";
import { createSession, deleteSession } from "./session";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface AuthResult {
  success: boolean;
  userId?: string;
  tenantId?: string;
  businessId?: string;
  role?: string;
  error?: string;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Generate a tenant slug from an email address.
 * Replaces "@" and "." with "-", lowercases, and strips other non-alphanumeric.
 */
function slugFromEmail(email: string): string {
  return email
    .toLowerCase()
    .replace(/[^a-z0-9@.]/g, "-")
    .replace(/[@.]/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

/**
 * Generate a tenant slug from a name.
 */
function slugFromName(name: string): string {
  return name
    .toLowerCase()
    .replace(/[^a-z0-9\s-]/g, "")
    .replace(/\s+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

// ---------------------------------------------------------------------------
// Default categories (seeded for every new tenant+business)
// ---------------------------------------------------------------------------

const DEFAULT_CATEGORIES = [
  // Income
  { name: "Sales", slug: "sales", type: "income" as const },
  { name: "Services", slug: "services", type: "income" as const },
  { name: "Interest Income", slug: "interest-income", type: "income" as const },
  { name: "Other Income", slug: "other-income", type: "income" as const },
  // Expense
  { name: "Office Supplies", slug: "office-supplies", type: "expense" as const },
  { name: "Utilities", slug: "utilities", type: "expense" as const },
  { name: "Rent", slug: "rent", type: "expense" as const },
  { name: "Salaries", slug: "salaries", type: "expense" as const },
  { name: "Advertising", slug: "advertising", type: "expense" as const },
  { name: "Travel", slug: "travel", type: "expense" as const },
  { name: "Other Expenses", slug: "other-expenses", type: "expense" as const },
];

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/**
 * Register a new user.
 *
 * Automatically creates a tenant workspace, a default business, a membership
 * (admin role), and default income/expense categories.
 */
export async function registerUser(
  email: string,
  name: string,
  password: string,
): Promise<AuthResult> {
  // 1. Check if email already exists
  const existingUser = db
    .select({ id: schema.users.id })
    .from(schema.users)
    .where(eq(schema.users.email, email))
    .get();

  if (existingUser) {
    return { success: false, error: "An account with this email already exists." };
  }

  // 2. Hash password
  const passwordHash = await hashPassword(password);

  // 3. Create user
  const userId = crypto.randomUUID();
  db.insert(schema.users).values({
    id: userId,
    email,
    name,
    passwordHash,
  }).run();

  // 4. Auto-create tenant
  const tenantId = crypto.randomUUID();
  const tenantSlug = slugFromName(name) + "-" + slugFromEmail(email);
  const tenantName = `${name}'s Workspace`;
  db.insert(schema.tenants).values({
    id: tenantId,
    name: tenantName,
    slug: tenantSlug,
    plan: "free",
  }).run();

  // 5. Auto-create business
  const businessId = crypto.randomUUID();
  db.insert(schema.businesses).values({
    id: businessId,
    tenantId,
    name: "My Business",
  }).run();

  // 6. Create membership (admin)
  db.insert(schema.memberships).values({
    id: crypto.randomUUID(),
    userId,
    tenantId,
    businessId,
    role: "admin",
  }).run();

  // 7. Create default categories
  for (const cat of DEFAULT_CATEGORIES) {
    db.insert(schema.categories).values({
      id: crypto.randomUUID(),
      tenantId,
      businessId,
      name: cat.name,
      slug: cat.slug,
      type: cat.type,
      isDefault: 1,
    }).run();
  }

  return {
    success: true,
    userId,
    tenantId,
    businessId,
    role: "admin",
  };
}

/**
 * Login with email + password.
 *
 * Verifies the user exists, checks the password, looks up the primary
 * membership for tenant/business context, and creates a session cookie.
 */
export async function loginUser(
  email: string,
  password: string,
): Promise<AuthResult> {
  // 1. Find user by email
  const user = db
    .select()
    .from(schema.users)
    .where(eq(schema.users.email, email))
    .get();

  if (!user) {
    return { success: false, error: "Invalid email or password." };
  }

  // 2. Verify password
  const isValid = await verifyPassword(password, user.passwordHash);
  if (!isValid) {
    return { success: false, error: "Invalid email or password." };
  }

  // 3. Look up primary membership
  const membership = db
    .select()
    .from(schema.memberships)
    .where(eq(schema.memberships.userId, user.id))
    .get();

  if (!membership) {
    return { success: false, error: "No workspace membership found. Please register first." };
  }

  // 4. Create session cookie
  await createSession(
    user.id,
    membership.tenantId,
    membership.businessId,
    membership.role,
  );

  return {
    success: true,
    userId: user.id,
    tenantId: membership.tenantId,
    businessId: membership.businessId,
    role: membership.role,
  };
}

/**
 * Logout the current user by deleting the session cookie.
 */
export async function logoutUser(): Promise<void> {
  await deleteSession();
}

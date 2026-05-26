import Database from "better-sqlite3";
import { drizzle } from "drizzle-orm/better-sqlite3";
import * as schema from "./schema";
import { hash } from "bcryptjs";
import { DEFAULT_CATEGORIES } from "@/lib/akaunkemas-saas/schemas";

async function main() {
  const sqlite = new Database("./data/akaunkemas.db");
  sqlite.pragma("journal_mode = WAL");
  sqlite.pragma("foreign_keys = ON");

  const db = drizzle(sqlite, { schema });

  const tenantId = "demo-tenant-001";
  const businessId = "demo-business-001";
  const userId = "demo-user-001";

  // Create demo tenant
  db.insert(schema.tenants)
    .values({
      id: tenantId,
      name: "Demo Enterprise",
      slug: "demo",
      plan: "free",
    })
    .onConflictDoNothing()
    .run();

  // Create demo business
  db.insert(schema.businesses)
    .values({
      id: businessId,
      tenantId,
      name: "Demo Business",
      address: "Kuala Lumpur, Malaysia",
    })
    .onConflictDoNothing()
    .run();

  // Create demo user (password: "demo1234")
  const passwordHash = await hash("demo1234", 10);
  db.insert(schema.users)
    .values({
      id: userId,
      email: "demo@akaunkemas.my",
      name: "Demo User",
      passwordHash,
    })
    .onConflictDoNothing()
    .run();

  // Create membership
  const membershipId = crypto.randomUUID();
  db.insert(schema.memberships)
    .values({
      id: membershipId,
      userId,
      tenantId,
      businessId,
      role: "admin",
    })
    .onConflictDoNothing()
    .run();

  // Seed default categories for the demo tenant+business
  for (const cat of DEFAULT_CATEGORIES) {
    db.insert(schema.categories)
      .values({
        id: crypto.randomUUID(),
        tenantId,
        businessId,
        name: cat.name,
        slug: cat.slug,
        type: cat.type,
        isDefault: 1,
      })
      .onConflictDoNothing()
      .run();
  }

  console.log("Seed complete: demo@akaunkemas.my / demo1234");
  sqlite.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

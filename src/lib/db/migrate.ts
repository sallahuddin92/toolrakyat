import Database from "better-sqlite3";
import { drizzle } from "drizzle-orm/better-sqlite3";
import { migrate } from "drizzle-orm/better-sqlite3/migrator";
import fs from "node:fs";

fs.mkdirSync("./data", { recursive: true });
const sqlite = new Database("./data/akaunkemas.db");
sqlite.pragma("journal_mode = WAL");
sqlite.pragma("foreign_keys = ON");

const db = drizzle(sqlite);

async function main() {
  console.log("Running migrations...");
  await migrate(db, { migrationsFolder: "./src/lib/db/migrations" });
  console.log("Migrations complete.");
  sqlite.close();
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});

ALTER TABLE `receipts` ADD `status` text DEFAULT 'draft' NOT NULL;--> statement-breakpoint
ALTER TABLE `transactions` ADD `status` text DEFAULT 'draft' NOT NULL;--> statement-breakpoint
ALTER TABLE `transactions` ADD `import_hash` text;--> statement-breakpoint
CREATE UNIQUE INDEX `ux_transactions_tenant_business_import_hash` ON `transactions` (`tenant_id`,`business_id`,`import_hash`) WHERE "transactions"."import_hash" IS NOT NULL;
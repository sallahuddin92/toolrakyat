CREATE TABLE `accountant_packs` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`label` text NOT NULL,
	`period_start` text NOT NULL,
	`period_end` text NOT NULL,
	`status` text DEFAULT 'draft' NOT NULL,
	`notes` text DEFAULT '' NOT NULL,
	`file_url` text,
	`generated_at` text,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `idx_accountant_packs_tenant_business_status` ON `accountant_packs` (`tenant_id`,`business_id`,`status`);--> statement-breakpoint
CREATE TABLE `audit_logs` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`user_id` text NOT NULL,
	`event_type` text NOT NULL,
	`entity_type` text NOT NULL,
	`entity_id` text NOT NULL,
	`summary` text NOT NULL,
	`metadata` text DEFAULT '{}' NOT NULL,
	`created_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `idx_audit_logs_tenant_business_created` ON `audit_logs` (`tenant_id`,`business_id`,`created_at`);--> statement-breakpoint
CREATE INDEX `idx_audit_logs_tenant_business_event` ON `audit_logs` (`tenant_id`,`business_id`,`event_type`);--> statement-breakpoint
CREATE TABLE `businesses` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`name` text NOT NULL,
	`registration_number` text,
	`address` text DEFAULT '' NOT NULL,
	`phone` text DEFAULT '' NOT NULL,
	`email` text DEFAULT '' NOT NULL,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL,
	FOREIGN KEY (`tenant_id`) REFERENCES `tenants`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE TABLE `categories` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`type` text NOT NULL,
	`is_default` integer DEFAULT 0 NOT NULL,
	`parent_id` text,
	`created_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `idx_categories_tenant_business` ON `categories` (`tenant_id`,`business_id`);--> statement-breakpoint
CREATE UNIQUE INDEX `ux_categories_tenant_business_slug` ON `categories` (`tenant_id`,`business_id`,`slug`);--> statement-breakpoint
CREATE TABLE `memberships` (
	`id` text PRIMARY KEY NOT NULL,
	`user_id` text NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`role` text DEFAULT 'staff' NOT NULL,
	`invited_at` text NOT NULL,
	`accepted_at` text,
	FOREIGN KEY (`user_id`) REFERENCES `users`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`tenant_id`) REFERENCES `tenants`(`id`) ON UPDATE no action ON DELETE cascade,
	FOREIGN KEY (`business_id`) REFERENCES `businesses`(`id`) ON UPDATE no action ON DELETE cascade
);
--> statement-breakpoint
CREATE UNIQUE INDEX `ux_memberships_user_tenant_business` ON `memberships` (`user_id`,`tenant_id`,`business_id`);--> statement-breakpoint
CREATE TABLE `receipt_matches` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`transaction_id` text NOT NULL,
	`receipt_id` text NOT NULL,
	`match_type` text NOT NULL,
	`date_delta` integer DEFAULT 0 NOT NULL,
	`amount_delta` real DEFAULT 0 NOT NULL,
	`matched_by` text NOT NULL,
	`created_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `idx_receipt_matches_tenant_business` ON `receipt_matches` (`tenant_id`,`business_id`);--> statement-breakpoint
CREATE INDEX `idx_receipt_matches_transaction` ON `receipt_matches` (`transaction_id`);--> statement-breakpoint
CREATE INDEX `idx_receipt_matches_receipt` ON `receipt_matches` (`receipt_id`);--> statement-breakpoint
CREATE TABLE `receipts` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`date` text NOT NULL,
	`merchant` text NOT NULL,
	`amount` real NOT NULL,
	`payment_method` text DEFAULT 'cash' NOT NULL,
	`category_slug` text DEFAULT 'uncategorised' NOT NULL,
	`tax_amount` real DEFAULT 0 NOT NULL,
	`service_charge` real DEFAULT 0 NOT NULL,
	`notes` text DEFAULT '' NOT NULL,
	`image_ref` text,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `idx_receipts_tenant_business_date` ON `receipts` (`tenant_id`,`business_id`,`date`);--> statement-breakpoint
CREATE TABLE `tenants` (
	`id` text PRIMARY KEY NOT NULL,
	`name` text NOT NULL,
	`slug` text NOT NULL,
	`plan` text DEFAULT 'free' NOT NULL,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `tenants_slug_unique` ON `tenants` (`slug`);--> statement-breakpoint
CREATE TABLE `transactions` (
	`id` text PRIMARY KEY NOT NULL,
	`tenant_id` text NOT NULL,
	`business_id` text NOT NULL,
	`date` text NOT NULL,
	`description` text NOT NULL,
	`debit` real DEFAULT 0 NOT NULL,
	`credit` real DEFAULT 0 NOT NULL,
	`amount` real DEFAULT 0 NOT NULL,
	`balance` real,
	`category_slug` text DEFAULT 'uncategorised' NOT NULL,
	`is_reconciled` integer DEFAULT 0 NOT NULL,
	`notes` text DEFAULT '' NOT NULL,
	`source` text DEFAULT 'manual' NOT NULL,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE INDEX `idx_transactions_tenant_business_date` ON `transactions` (`tenant_id`,`business_id`,`date`);--> statement-breakpoint
CREATE INDEX `idx_transactions_tenant_business_category` ON `transactions` (`tenant_id`,`business_id`,`category_slug`);--> statement-breakpoint
CREATE TABLE `users` (
	`id` text PRIMARY KEY NOT NULL,
	`email` text NOT NULL,
	`name` text NOT NULL,
	`password_hash` text NOT NULL,
	`created_at` text NOT NULL,
	`updated_at` text NOT NULL
);
--> statement-breakpoint
CREATE UNIQUE INDEX `users_email_unique` ON `users` (`email`);
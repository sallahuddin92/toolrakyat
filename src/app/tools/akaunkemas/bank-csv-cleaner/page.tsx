import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { RecentlyUsedTracker } from "@/components/tools/RecentlyUsedTracker";
import { ToolPageShell } from "@/components/tools/ToolPageShell";
import { BankCsvCleanerTool } from "@/components/tools/implementations/akaunkemas/BankCsvCleanerTool";
import { getToolByCategoryAndSlug } from "@/lib/tools/registry";

export const metadata: Metadata = (() => {
  const tool = getToolByCategoryAndSlug("akaunkemas", "bank-csv-cleaner");
  if (!tool) return {};
  return { title: tool.seoTitle, description: tool.seoDescription };
})();

export default function BankCsvCleanerPage() {
  const tool = getToolByCategoryAndSlug("akaunkemas", "bank-csv-cleaner");
  if (!tool) notFound();

  return (
    <ToolPageShell
      title={tool.name}
      description={tool.description}
      categoryId={tool.categoryId}
      toolId={tool.id}
      privacyNote={tool.privacyNote}
      acceptedFileTypes={tool.acceptedFileTypes}
      maxFileSizeMB={tool.maxFileSizeMB}
    >
      <RecentlyUsedTracker toolId={tool.id} />
      <BankCsvCleanerTool tool={tool} />
    </ToolPageShell>
  );
}

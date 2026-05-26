import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { RecentlyUsedTracker } from "@/components/tools/RecentlyUsedTracker";
import { ToolPageShell } from "@/components/tools/ToolPageShell";
import { ReceiptMatcherTool } from "@/components/tools/implementations/akaunkemas/ReceiptMatcherTool";
import { getToolByCategoryAndSlug } from "@/lib/tools/registry";

export const metadata: Metadata = (() => {
  const tool = getToolByCategoryAndSlug("akaunkemas", "receipt-matcher");
  if (!tool) return {};
  return { title: tool.seoTitle, description: tool.seoDescription };
})();

export default function ReceiptMatcherPage() {
  const tool = getToolByCategoryAndSlug("akaunkemas", "receipt-matcher");
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
      <ReceiptMatcherTool tool={tool} />
    </ToolPageShell>
  );
}

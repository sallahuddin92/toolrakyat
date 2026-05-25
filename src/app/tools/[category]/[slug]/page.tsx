import type React from "react";
import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { RecentlyUsedTracker } from "@/components/tools/RecentlyUsedTracker";
import { ToolPageShell } from "@/components/tools/ToolPageShell";
import { ToolPlaceholder } from "@/components/tools/ToolPlaceholder";
import { WordCounterTool } from "@/components/tools/implementations/text/WordCounterTool";
import { getToolByCategoryAndSlug } from "@/lib/tools/registry";
import { TOOL_CATEGORIES, type ToolCategoryId } from "@/lib/tools/types";

function isCategoryId(value: string): value is ToolCategoryId {
  return TOOL_CATEGORIES.some((c) => c.id === value);
}

export function generateMetadata({
  params,
}: {
  params: Promise<{ category: string; slug: string }>;
}): Promise<Metadata> {
  return (async () => {
    const { category, slug } = await params;
    if (!isCategoryId(category)) return {};
    const tool = getToolByCategoryAndSlug(category, slug);
    if (!tool) return {};

    return {
      title: tool.seoTitle,
      description: tool.seoDescription,
    };
  })();
}

export default async function ToolDetailPage({
  params,
}: {
  params: Promise<{ category: string; slug: string }>;
}) {
  const { category, slug } = await params;
  if (!isCategoryId(category)) notFound();

  const tool = getToolByCategoryAndSlug(category, slug);
  if (!tool) notFound();

  let content: React.ReactNode = null;
  if (tool.id === "text-word-counter") {
    content = <WordCounterTool />;
  } else if (!tool.isImplemented) {
    content = (
      <ToolPlaceholder
        title="Coming soon"
        note="We ship tool UIs early, then wire up safe local/server-side processing. Check back soon."
      />
    );
  } else {
    content = (
      <ToolPlaceholder
        title="Tool wiring pending"
        note="This tool is marked implemented, but its UI wiring isn't added yet."
      />
    );
  }

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
      {content}
    </ToolPageShell>
  );
}

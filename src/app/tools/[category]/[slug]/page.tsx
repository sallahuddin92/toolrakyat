import type React from "react";
import type { Metadata } from "next";
import { notFound } from "next/navigation";

import { RecentlyUsedTracker } from "@/components/tools/RecentlyUsedTracker";
import { ToolPageShell } from "@/components/tools/ToolPageShell";
import { ToolPlaceholder } from "@/components/tools/ToolPlaceholder";
import { GenericUtilityTool } from "@/components/tools/implementations/GenericUtilityTool";
import { WordCounterTool } from "@/components/tools/implementations/text/WordCounterTool";
import { SmartPdfLaunchCard } from "@/components/tools/implementations/pdf/SmartPdfLaunchCard";
import { getImplementationKey } from "@/lib/tools/implementation-registry";
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

  const implementation = getImplementationKey(tool);
  let content: React.ReactNode;
  if (implementation === "word-counter") {
    content = <WordCounterTool />;
  } else if (implementation === "smartpdf-launcher") {
    content = <SmartPdfLaunchCard />;
  } else if (implementation === "generic-utility") {
    content = <GenericUtilityTool tool={tool} />;
  } else if (!tool.isImplemented) {
    content = (
      <ToolPlaceholder
        title="Coming soon"
        note="We ship tool UIs early, then wire up safe local/server-side processing. Check back soon."
      />
    );
  } else notFound();

  return (
    <ToolPageShell
      title={tool.name}
      description={tool.description}
      categoryId={tool.categoryId}
      toolId={tool.id}
      privacyNote={tool.id === "pdf-editor" ? undefined : tool.privacyNote}
      acceptedFileTypes={tool.acceptedFileTypes}
      maxFileSizeMB={tool.maxFileSizeMB}
    >
      <RecentlyUsedTracker toolId={tool.id} />
      {content}
    </ToolPageShell>
  );
}

import type { Metadata } from "next";
import Link from "next/link";

import { Container } from "@/components/layout/Container";
import { ToolCard } from "@/components/tools/ToolCard";
import { Button } from "@/components/ui/button";
import { getCategoryLabel, getToolsByCategory } from "@/lib/tools/registry";
import type { ToolCategoryId } from "@/lib/tools/types";

export function categoryMetadata(categoryId: ToolCategoryId): Metadata {
  const label = getCategoryLabel(categoryId);
  return {
    title: `${label} Tools`,
    description: `Browse ${label} tools on ToolRakyat. Privacy-first processing and fast, practical utilities.`,
  };
}

export function CategoryPage({ categoryId }: { categoryId: ToolCategoryId }) {
  const label = getCategoryLabel(categoryId);
  const list = getToolsByCategory(categoryId);

  return (
    <div className="bg-gradient-to-b from-white to-slate-50">
      <Container className="py-10 sm:py-14">
        <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h1 className="text-3xl font-semibold tracking-tight text-slate-900 sm:text-4xl">
              {label} tools
            </h1>
            <p className="mt-3 max-w-2xl text-base leading-7 text-slate-600">
              {list.length} tool{list.length === 1 ? "" : "s"} in this category.
            </p>
          </div>
          <Button asChild variant="secondary" className="rounded-2xl">
            <Link href="/tools">Browse all tools</Link>
          </Button>
        </div>

        <div className="mt-8 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {list.map((tool) => (
            <ToolCard key={tool.id} tool={tool} />
          ))}
        </div>
      </Container>
    </div>
  );
}


import Link from "next/link";

import { ToolCard } from "@/components/tools/ToolCard";
import { Button } from "@/components/ui/button";
import { Separator } from "@/components/ui/separator";
import { getToolsByCategory, getCategoryLabel } from "@/lib/tools/registry";
import type { ToolCategoryId, ToolDefinition } from "@/lib/tools/types";

function pickRelated(
  categoryId: ToolCategoryId,
  currentToolId: string,
  limit: number,
): ToolDefinition[] {
  const list = getToolsByCategory(categoryId).filter((t) => t.id !== currentToolId);
  const implemented = list.filter((t) => t.isImplemented);
  const rest = list.filter((t) => !t.isImplemented);
  return [...implemented, ...rest].slice(0, limit);
}

export function RelatedTools({
  categoryId,
  currentToolId,
  limit = 6,
}: {
  categoryId: ToolCategoryId;
  currentToolId: string;
  limit?: number;
}) {
  const label = getCategoryLabel(categoryId);
  const related = pickRelated(categoryId, currentToolId, limit);

  if (!related.length) return null;

  return (
    <div className="pt-10">
      <Separator className="mb-8" />
      <div className="flex items-end justify-between gap-4">
        <div>
          <div className="text-lg font-semibold tracking-tight text-slate-900">
            Related {label} tools
          </div>
          <div className="mt-1 text-sm text-slate-600">
            More tools you might find useful.
          </div>
        </div>
        <Button asChild variant="secondary" className="rounded-2xl">
          <Link href={`/tools/${categoryId}`}>Browse {label}</Link>
        </Button>
      </div>

      <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
        {related.map((tool) => (
          <ToolCard key={tool.id} tool={tool} />
        ))}
      </div>
    </div>
  );
}


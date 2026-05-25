import Link from "next/link";

import { Card, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ToolIcon } from "@/components/tools/ToolIcon";
import { Badge } from "@/components/ui/badge";
import type { ToolDefinition } from "@/lib/tools/types";

export function ToolCard({ tool }: { tool: ToolDefinition }) {
  return (
    <Link href={tool.route} className="group block">
      <Card className="h-full rounded-2xl transition-shadow group-hover:shadow-md">
        <CardHeader className="space-y-2">
          <div className="flex items-start justify-between gap-3">
            <div className="flex items-center gap-3">
              <div className="grid size-10 place-items-center rounded-2xl bg-slate-50 text-slate-900 ring-1 ring-slate-200">
                <ToolIcon name={tool.icon} className="size-5" />
              </div>
              <div>
                <CardTitle className="text-base text-slate-900">
                  {tool.name}
                </CardTitle>
                <CardDescription className="text-sm text-slate-600">
                  {tool.description}
                </CardDescription>
              </div>
            </div>
            {!tool.isImplemented ? (
              <Badge variant="secondary" className="rounded-full">
                Coming soon
              </Badge>
            ) : null}
          </div>

          <div className="flex flex-wrap gap-1.5 pt-1">
            {tool.tags.slice(0, 3).map((tag) => (
              <span
                key={tag}
                className="rounded-full bg-slate-100 px-2 py-0.5 text-xs text-slate-600"
              >
                {tag}
              </span>
            ))}
          </div>
        </CardHeader>
      </Card>
    </Link>
  );
}


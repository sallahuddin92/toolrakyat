"use client";

import { useMemo, useState } from "react";
import { useRouter, useSearchParams } from "next/navigation";

import { ToolCard } from "@/components/tools/ToolCard";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Separator } from "@/components/ui/separator";
import { tools } from "@/lib/tools/registry";
import { TOOL_CATEGORIES, type ToolCategoryId } from "@/lib/tools/types";

function normalize(s: string) {
  return s.trim().toLowerCase();
}

export function ToolDirectory() {
  const router = useRouter();
  const params = useSearchParams();

  const initialQuery = params.get("q") ?? "";
  const initialCategory = (params.get("category") ?? "") as ToolCategoryId | "";
  const initialPopular = params.get("popular") === "1";

  const [query, setQuery] = useState(initialQuery);
  const [categoryId, setCategoryId] = useState<ToolCategoryId | "">(initialCategory);
  const [popularOnly, setPopularOnly] = useState(initialPopular);

  const filtered = useMemo(() => {
    const q = normalize(query);
    return tools.filter((t) => {
      if (popularOnly && !t.isPopular) return false;
      if (categoryId && t.categoryId !== categoryId) return false;
      if (!q) return true;

      const haystack = normalize(
        [t.name, t.description, t.category, ...t.tags].join(" "),
      );
      return haystack.includes(q);
    });
  }, [query, categoryId, popularOnly]);

  function syncUrl(next: {
    q?: string;
    category?: string;
    popular?: boolean;
  }) {
    const url = new URL(window.location.href);
    url.searchParams.delete("q");
    url.searchParams.delete("category");
    url.searchParams.delete("popular");

    if (next.q && next.q.trim()) url.searchParams.set("q", next.q.trim());
    if (next.category) url.searchParams.set("category", next.category);
    if (next.popular) url.searchParams.set("popular", "1");

    router.replace(url.pathname + url.search);
  }

  return (
    <div className="space-y-6">
      <div className="grid gap-3 md:grid-cols-3">
        <div className="md:col-span-2">
          <Input
            value={query}
            onChange={(e) => {
              const v = e.target.value;
              setQuery(v);
              syncUrl({ q: v, category: categoryId, popular: popularOnly });
            }}
            placeholder="Search tools (e.g., merge PDF, compress image, invoice...)"
            className="h-12 rounded-2xl bg-white"
          />
        </div>
        <Select
          value={categoryId || "all"}
          onValueChange={(v) => {
            const next = v === "all" ? "" : (v as ToolCategoryId);
            setCategoryId(next);
            syncUrl({ q: query, category: next, popular: popularOnly });
          }}
        >
          <SelectTrigger className="h-12 rounded-2xl bg-white">
            <SelectValue placeholder="Category" />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">All categories</SelectItem>
            {TOOL_CATEGORIES.map((c) => (
              <SelectItem key={c.id} value={c.id}>
                {c.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>

      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div className="flex items-center gap-2 text-sm text-slate-600">
          <Checkbox
            id="popular-only"
            checked={popularOnly}
            onCheckedChange={(v) => {
              const next = Boolean(v);
              setPopularOnly(next);
              syncUrl({ q: query, category: categoryId, popular: next });
            }}
          />
          <label htmlFor="popular-only">Popular tools only</label>
        </div>
        <div className="text-sm text-slate-600">
          Showing <span className="font-medium text-slate-900">{filtered.length}</span>{" "}
          tool{filtered.length === 1 ? "" : "s"}
        </div>
      </div>

      <Separator />

      {filtered.length === 0 ? (
        <div className="rounded-2xl border bg-white p-8 text-center text-slate-600">
          No tools found. Try a different keyword or category.
        </div>
      ) : (
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {filtered.map((tool) => (
            <ToolCard key={tool.id} tool={tool} />
          ))}
        </div>
      )}
    </div>
  );
}


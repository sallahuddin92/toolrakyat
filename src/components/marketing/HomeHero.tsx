"use client";

import { useMemo, useState } from "react";
import { useRouter } from "next/navigation";

import { Container } from "@/components/layout/Container";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { TOOL_CATEGORIES } from "@/lib/tools/types";

export function HomeHero() {
  const router = useRouter();
  const [query, setQuery] = useState("");

  const placeholder = useMemo(() => {
    return "Search tools like merge PDF, compress image, invoice...";
  }, []);

  return (
    <section className="relative overflow-hidden bg-gradient-to-b from-white to-slate-50">
      <div className="pointer-events-none absolute -top-24 right-[-120px] size-[380px] rounded-full bg-gradient-to-br from-sky-500/20 via-violet-500/20 to-emerald-500/20 blur-3xl" />
      <div className="pointer-events-none absolute -bottom-28 left-[-120px] size-[420px] rounded-full bg-gradient-to-br from-emerald-500/20 via-sky-500/20 to-violet-500/20 blur-3xl" />

      <Container className="relative py-14 sm:py-20">
        <div className="mx-auto max-w-3xl text-center">
          <div className="inline-flex items-center rounded-full bg-slate-100 px-3 py-1 text-xs text-slate-600">
            Privacy-first. Temporary processing. No accounts required.
          </div>

          <h1 className="mt-5 text-4xl font-semibold tracking-tight text-slate-900 sm:text-5xl">
            Free Online Productivity Tools
          </h1>
          <p className="mt-4 text-base leading-7 text-slate-600 sm:text-lg">
            PDF, image, compression, converter, business, calculator, and
            developer tools in one place.
          </p>

          <form
            className="mt-7 flex flex-col gap-3 sm:flex-row"
            onSubmit={(e) => {
              e.preventDefault();
              const trimmed = query.trim();
              router.push(trimmed ? `/tools?q=${encodeURIComponent(trimmed)}` : "/tools");
            }}
          >
            <Input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={placeholder}
              className="h-12 rounded-2xl bg-white"
            />
            <Button type="submit" className="h-12 rounded-2xl">
              Search tools
            </Button>
            <Button
              type="button"
              variant="secondary"
              className="h-12 rounded-2xl"
              onClick={() => router.push("/tools")}
            >
              Browse all tools
            </Button>
          </form>

          <div className="mt-6 flex flex-wrap justify-center gap-2">
            {TOOL_CATEGORIES.map((c) => (
              <button
                key={c.id}
                type="button"
                className="rounded-full border bg-white px-3 py-1 text-xs text-slate-700 shadow-sm hover:bg-slate-50"
                onClick={() => router.push(`/tools?category=${encodeURIComponent(c.id)}`)}
              >
                {c.label}
              </button>
            ))}
          </div>
        </div>
      </Container>
    </section>
  );
}


"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { Container } from "@/components/layout/Container";
import { Button } from "@/components/ui/button";

export function SiteHeader() {
  const pathname = usePathname();
  if (pathname?.startsWith("/smartpdf")) {
    return null;
  }

  return (
    <header className="sticky top-0 z-40 border-b bg-background/80 backdrop-blur">
      <Container className="flex h-16 items-center justify-between">
        <Link href="/" className="flex items-center gap-2">
          <div className="grid size-9 place-items-center rounded-xl bg-gradient-to-br from-sky-500 via-violet-500 to-emerald-500 text-white shadow-sm">
            <span className="text-sm font-semibold">TR</span>
          </div>
          <div className="leading-tight">
            <div className="text-sm font-semibold text-slate-900">
              ToolRakyat
            </div>
            <div className="text-xs text-slate-500">
              Free practical productivity tools
            </div>
          </div>
        </Link>

        <nav className="flex items-center gap-2">
          <Button asChild variant="ghost" className="hidden sm:inline-flex">
            <Link href="/tools">Tools</Link>
          </Button>
          <Button asChild variant="ghost" className="hidden sm:inline-flex">
            <Link href="/privacy">Privacy</Link>
          </Button>
          <Button asChild variant="ghost" className="hidden sm:inline-flex">
            <Link href="/terms">Terms</Link>
          </Button>
          <Button asChild variant="secondary" className="rounded-full">
            <Link href="/tools">Browse tools</Link>
          </Button>
        </nav>
      </Container>
    </header>
  );
}


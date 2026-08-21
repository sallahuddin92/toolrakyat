"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";

import { Container } from "@/components/layout/Container";
import { Separator } from "@/components/ui/separator";

export function SiteFooter() {
  const pathname = usePathname();
  if (pathname?.startsWith("/smartpdf")) {
    return null;
  }

  return (
    <footer className="border-t bg-background">
      <Container className="py-10">
        <div className="flex flex-col gap-6 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <div className="text-sm font-semibold text-slate-900">ToolRakyat</div>
            <div className="mt-1 max-w-md text-sm text-slate-600">
              Free practical productivity tools for everyone. Privacy-first
              processing with temporary files.
            </div>
          </div>
          <div className="flex flex-wrap gap-4 text-sm text-slate-600">
            <Link href="/tools" className="hover:text-slate-900">
              Tools
            </Link>
            <Link href="/privacy" className="hover:text-slate-900">
              Privacy
            </Link>
            <Link href="/terms" className="hover:text-slate-900">
              Terms
            </Link>
            <Link href="/pricing" className="hover:text-slate-900">
              Pricing
            </Link>
          </div>
        </div>

        <Separator className="my-8" />

        <div className="flex flex-col gap-2 text-xs text-slate-500 sm:flex-row sm:items-center sm:justify-between">
          <div>© {new Date().getFullYear()} ToolRakyat. All rights reserved.</div>
          <div>
            Built for students, SMEs, freelancers, and teams — Malaysia-friendly,
            global usable.
          </div>
        </div>
      </Container>
    </footer>
  );
}


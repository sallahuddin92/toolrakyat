"use client";

import { useState } from "react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  LayoutDashboard,
  ArrowLeftRight,
  Receipt,
  GitMerge,
  Package,
  ScrollText,
  Settings,
  Menu,
  X,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

const navLinks = [
  { href: "/app/akaunkemas", label: "Dashboard", icon: LayoutDashboard },
  { href: "/app/akaunkemas/transactions", label: "Transactions", icon: ArrowLeftRight },
  { href: "/app/akaunkemas/receipts", label: "Receipts", icon: Receipt },
  { href: "/app/akaunkemas/matching", label: "Matching", icon: GitMerge },
  { href: "/app/akaunkemas/accountant-packs", label: "Accountant Packs", icon: Package },
  { href: "/app/akaunkemas/audit-logs", label: "Audit Logs", icon: ScrollText },
  { href: "/app/akaunkemas/settings/business", label: "Settings", icon: Settings },
];

export function SidebarLayout({ children }: { children: React.ReactNode }) {
  const pathname = usePathname();
  const [mobileOpen, setMobileOpen] = useState(false);

  const sidebarContent = (
    <nav className="flex flex-col gap-1 px-3 py-4">
      {navLinks.map((link) => {
        const Icon = link.icon;
        const isActive =
          link.href === "/app/akaunkemas"
            ? pathname === "/app/akaunkemas"
            : pathname.startsWith(link.href);
        return (
          <Link
            key={link.href}
            href={link.href}
            onClick={() => setMobileOpen(false)}
            className={cn(
              "flex items-center gap-3 rounded-xl px-3 py-2 text-sm font-medium transition-colors",
              isActive
                ? "bg-sky-100 text-sky-700"
                : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
            )}
          >
            <Icon className="size-4 shrink-0" />
            {link.label}
          </Link>
        );
      })}
    </nav>
  );

  return (
    <div className="flex min-h-screen">
      {/* Desktop sidebar */}
      <aside className="hidden lg:flex lg:w-64 lg:flex-col lg:border-r lg:bg-white">
        <div className="flex h-14 items-center gap-2 border-b px-4">
          <Package className="size-5 text-sky-600" />
          <span className="font-semibold text-slate-900">AkaunKemas</span>
          <Badge variant="outline" className="ml-auto text-[10px]">
            DEV MODE
          </Badge>
        </div>
        {sidebarContent}
        <div className="mt-auto px-3 py-4">
          <Separator className="mb-4" />
          <p className="px-3 text-[11px] text-slate-400">
            DEV MODE — No real auth
          </p>
        </div>
      </aside>

      {/* Mobile top bar + sidebar overlay */}
      <div className="flex flex-1 flex-col lg:hidden">
        <div className="flex h-14 items-center gap-2 border-b bg-white px-4">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setMobileOpen(!mobileOpen)}
            aria-label="Toggle menu"
          >
            {mobileOpen ? <X className="size-4" /> : <Menu className="size-4" />}
          </Button>
          <Package className="size-5 text-sky-600" />
          <span className="font-semibold text-slate-900">AkaunKemas</span>
          <Badge variant="outline" className="ml-auto text-[10px]">
            DEV MODE
          </Badge>
        </div>

        {/* Mobile nav overlay */}
        {mobileOpen && (
          <div className="fixed inset-0 top-14 z-40 bg-black/20" onClick={() => setMobileOpen(false)}>
            <div
              className="h-full w-64 border-r bg-white shadow-lg"
              onClick={(e) => e.stopPropagation()}
            >
              {sidebarContent}
              <div className="mt-auto px-3 py-4">
                <Separator className="mb-4" />
                <p className="px-3 text-[11px] text-slate-400">
                  DEV MODE — No real auth
                </p>
              </div>
            </div>
          </div>
        )}

        <main className="flex-1 p-4 lg:p-6">{children}</main>
      </div>

      {/* Desktop main content */}
      <div className="hidden lg:flex lg:flex-1 lg:flex-col">
        <main className="flex-1 p-6">{children}</main>
      </div>
    </div>
  );
}

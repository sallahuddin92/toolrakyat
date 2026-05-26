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
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { cn } from "@/lib/utils";
import { TopBar } from "./TopBar";

// ---------------------------------------------------------------------------
// Navigation links
// ---------------------------------------------------------------------------

interface NavLink {
  href: string;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const navLinks: NavLink[] = [
  { href: "/app/akaunkemas", label: "Dashboard", icon: LayoutDashboard },
  { href: "/app/akaunkemas/transactions", label: "Transactions", icon: ArrowLeftRight },
  { href: "/app/akaunkemas/receipts", label: "Receipts", icon: Receipt },
  { href: "/app/akaunkemas/matching", label: "Matching", icon: GitMerge },
  { href: "/app/akaunkemas/accountant-packs", label: "Accountant Packs", icon: Package },
  { href: "/app/akaunkemas/audit-logs", label: "Audit Logs", icon: ScrollText },
];

const settingsLink: NavLink = {
  href: "/app/akaunkemas/settings/business",
  label: "Settings",
  icon: Settings,
};

// ---------------------------------------------------------------------------
// SidebarNav component
// ---------------------------------------------------------------------------

function SidebarNav({
  onNavigate,
}: {
  onNavigate?: () => void;
}) {
  const pathname = usePathname();

  const isActive = (href: string) => {
    if (href === "/app/akaunkemas") {
      return pathname === "/app/akaunkemas";
    }
    return pathname.startsWith(href);
  };

  return (
    <nav className="flex flex-col gap-1 px-3 py-4">
      {navLinks.map((link) => {
        const Icon = link.icon;
        const active = isActive(link.href);
        return (
          <Link
            key={link.href}
            href={link.href}
            onClick={onNavigate}
            className={cn(
              "flex items-center gap-3 rounded-xl px-3 py-2 text-sm font-medium transition-colors duration-150",
              "border-l-[3px] border-l-transparent",
              active
                ? "border-l-sky-600 bg-sky-50 text-sky-600"
                : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
            )}
          >
            <Icon className="size-4 shrink-0" />
            {link.label}
          </Link>
        );
      })}

      <Separator className="my-2" />

      {/* Settings */}
      {(() => {
        const active = isActive(settingsLink.href);
        const Icon = settingsLink.icon;
        return (
          <Link
            href={settingsLink.href}
            onClick={onNavigate}
            className={cn(
              "flex items-center gap-3 rounded-xl px-3 py-2 text-sm font-medium transition-colors duration-150",
              "border-l-[3px] border-l-transparent",
              active
                ? "border-l-sky-600 bg-sky-50 text-sky-600"
                : "text-slate-600 hover:bg-slate-100 hover:text-slate-900"
            )}
          >
            <Icon className="size-4 shrink-0" />
            Settings
          </Link>
        );
      })()}
    </nav>
  );
}

// ---------------------------------------------------------------------------
// SidebarFooter component
// ---------------------------------------------------------------------------

function SidebarFooter() {
  const isDemoMode = process.env.NEXT_PUBLIC_DEMO_MODE === "true";
  const showDevBadge = isDemoMode || process.env.NODE_ENV === "development";

  return (
    <div className="mt-auto px-3 py-4">
      <Separator className="mb-4" />
      <div className="flex flex-col gap-2 px-3">
        {showDevBadge && (
          <Badge variant="outline" className="w-fit text-[10px]">
            DEV MODE
          </Badge>
        )}
        <p className="text-[11px] text-slate-400">
          AkaunKemas v0.2
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// SidebarLayout component
// ---------------------------------------------------------------------------

export function SidebarLayout({ children }: { children: React.ReactNode }) {
  const [mobileOpen, setMobileOpen] = useState(false);

  const closeMobile = () => setMobileOpen(false);

  return (
    <div className="flex min-h-screen bg-slate-50">
      {/* Desktop sidebar */}
      <aside className="hidden lg:flex lg:w-60 lg:shrink-0 lg:flex-col lg:border-r lg:border-slate-200 lg:bg-white">
        {/* Logo / brand */}
        <div className="flex h-14 items-center gap-2 border-b border-slate-200 px-4">
          <Package className="size-5 text-sky-600" />
          <span className="font-semibold text-slate-900">AkaunKemas</span>
        </div>

        {/* Navigation */}
        <SidebarNav />

        {/* Footer */}
        <SidebarFooter />
      </aside>

      {/* Main content area (mobile + desktop) */}
      <div className="flex flex-1 flex-col min-w-0">
        {/* Top bar (always visible) */}
        <TopBar
          mobileOpen={mobileOpen}
          onToggleMobile={() => setMobileOpen((prev) => !prev)}
        />

        {/* Mobile sidebar overlay */}
        {mobileOpen && (
          <div
            className="fixed inset-0 top-14 z-40 bg-black/30 lg:hidden"
            onClick={closeMobile}
          >
            <div
              className="h-full w-60 border-r border-slate-200 bg-white shadow-lg"
              onClick={(e) => e.stopPropagation()}
            >
              {/* Mobile sidebar header */}
              <div className="flex h-14 items-center gap-2 border-b border-slate-200 px-4">
                <Package className="size-5 text-sky-600" />
                <span className="font-semibold text-slate-900">AkaunKemas</span>
              </div>

              <SidebarNav onNavigate={closeMobile} />
              <SidebarFooter />
            </div>
          </div>
        )}

        {/* Page content */}
        <main className="flex-1 overflow-auto p-4 lg:p-6">{children}</main>
      </div>
    </div>
  );
}

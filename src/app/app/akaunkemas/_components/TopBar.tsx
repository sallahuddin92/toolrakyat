"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import {
  Package,
  Menu,
  X,
  LogOut,
  ChevronRight,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";

// ---------------------------------------------------------------------------
// Demo user data (matches demo mode in src/lib/auth/dal.ts)
// ---------------------------------------------------------------------------

const DEMO_USER = {
  name: "Demo User",
  email: "demo@akaunkemas.dev",
  role: "admin",
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function breadcrumbLabel(pathname: string): string {
  const segments = pathname.split("/").filter(Boolean);
  // Routes: /app/akaunkemas/... -> skip "app", "akaunkemas"
  const routeSegments = segments.slice(2);

  if (routeSegments.length === 0) return "Dashboard";

  const mapping: Record<string, string> = {
    transactions: "Transactions",
    receipts: "Receipts",
    matching: "Matching",
    "accountant-packs": "Accountant Packs",
    "audit-logs": "Audit Logs",
    settings: "Settings",
    business: "Business",
    members: "Members",
    billing: "Billing",
  };

  return routeSegments
    .map((s) => mapping[s] ?? s.charAt(0).toUpperCase() + s.slice(1))
    .join(" / ");
}

function pageTitle(pathname: string): string {
  const label = breadcrumbLabel(pathname);
  const parts = label.split(" / ");
  return parts[parts.length - 1];
}

// ---------------------------------------------------------------------------
// Avatar
// ---------------------------------------------------------------------------

function UserAvatar({ name }: { name: string }) {
  const initial = (name ?? "U").charAt(0).toUpperCase();
  return (
    <div
      className="flex size-8 shrink-0 items-center justify-center rounded-full bg-sky-100 text-xs font-semibold text-sky-700"
      aria-hidden="true"
    >
      {initial}
    </div>
  );
}

// ---------------------------------------------------------------------------
// TopBar component
// ---------------------------------------------------------------------------

interface TopBarProps {
  mobileOpen: boolean;
  onToggleMobile: () => void;
}

export function TopBar({ mobileOpen, onToggleMobile }: TopBarProps) {
  const pathname = usePathname();
  const isDemoMode = process.env.NEXT_PUBLIC_DEMO_MODE === "true";
  const showDemoBadge = isDemoMode || process.env.NODE_ENV === "development";

  const handleSignOut = () => {
    // Redirect to logout route which clears the session cookie
    window.location.href = "/app/akaunkemas/logout";
  };

  return (
    <header className="flex h-14 shrink-0 items-center gap-3 border-b border-slate-200 bg-white px-4">
      {/* Mobile hamburger */}
      <Button
        variant="ghost"
        size="icon"
        className="lg:hidden"
        onClick={onToggleMobile}
        aria-label={mobileOpen ? "Close menu" : "Open menu"}
      >
        {mobileOpen ? (
          <X className="size-4" />
        ) : (
          <Menu className="size-4" />
        )}
      </Button>

      {/* Brand (visible when sidebar is hidden on mobile) */}
      <Link
        href="/app/akaunkemas"
        className="flex items-center gap-2 lg:hidden"
      >
        <Package className="size-5 text-sky-600" />
        <span className="font-semibold text-slate-900">AkaunKemas</span>
      </Link>

      {/* Breadcrumb */}
      <nav
        className="hidden items-center gap-1.5 text-sm text-slate-500 sm:flex"
        aria-label="Breadcrumb"
      >
        <Link
          href="/app/akaunkemas"
          className="transition-colors duration-150 hover:text-sky-600"
        >
          Home
        </Link>
        <ChevronRight className="size-3.5 text-slate-400" />
        <span className="font-medium text-slate-700">
          {pageTitle(pathname)}
        </span>
      </nav>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Right section: user info */}
      <div className="flex items-center gap-2">
        {/* DEV MODE badge (always show in dev) */}
        {showDemoBadge && (
          <Badge variant="outline" className="text-[10px]">
            DEV MODE
          </Badge>
        )}

        <Separator orientation="vertical" className="h-6" />

        {/* User avatar + name (hidden on very small screens) */}
        <div className="hidden items-center gap-2 sm:flex">
          <UserAvatar name={DEMO_USER.name} />
          <div className="flex flex-col leading-tight">
            <span className="text-xs font-medium text-slate-700">
              {DEMO_USER.name}
            </span>
            <Badge
              variant="secondary"
              className="h-4 px-1.5 text-[10px] capitalize"
            >
              {DEMO_USER.role}
            </Badge>
          </div>
        </div>

        {/* Sign Out */}
        <Button
          variant="ghost"
          size="icon"
          onClick={handleSignOut}
          aria-label="Sign out"
          title="Sign out"
        >
          <LogOut className="size-4 text-slate-500" />
        </Button>
      </div>
    </header>
  );
}

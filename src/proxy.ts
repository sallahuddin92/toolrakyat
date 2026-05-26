import { NextResponse } from "next/server";
import type { NextRequest } from "next/server";
import { LIMITS } from "@/lib/limits";
import { decrypt } from "@/lib/auth/session";

/**
 * Simple in-memory rate limiter for ToolRakyat production.
 * Note: Resets on server restart. For multi-node, use Redis.
 */
const rateLimitStore = new Map<string, { tokens: number; lastRefill: number }>();

function isLocalPlaywrightRequest(request: NextRequest) {
  const isLocalhost =
    request.nextUrl.hostname === "localhost" ||
    request.nextUrl.hostname === "127.0.0.1" ||
    request.nextUrl.hostname === "::1";

  return (
    isLocalhost &&
    request.headers.get("x-toolrakyat-e2e") === "playwright"
  );
}

function getRateLimit(ip: string) {
  const now = Date.now();
  const config = {
    limit: LIMITS.RATE_LIMIT_MAX,
    burst: LIMITS.RATE_LIMIT_BURST,
    windowMs: 60 * 1000,
  };

  let record = rateLimitStore.get(ip);

  if (!record) {
    record = { tokens: config.limit + config.burst, lastRefill: now };
    rateLimitStore.set(ip, record);
    return { ok: true, remaining: record.tokens };
  }

  // Refill tokens based on time passed
  const timePassed = now - record.lastRefill;
  const refillAmount = (timePassed / config.windowMs) * config.limit;
  
  record.tokens = Math.min(
    config.limit + config.burst,
    record.tokens + refillAmount
  );
  record.lastRefill = now;

  if (record.tokens >= 1) {
    record.tokens -= 1;
    return { ok: true, remaining: Math.floor(record.tokens) };
  }

  return { ok: false, remaining: 0 };
}

export async function proxy(request: NextRequest) {
  const ip = request.headers.get("x-forwarded-for")?.split(",")[0] ?? "127.0.0.1";
  const { pathname } = request.nextUrl;

  // -----------------------------------------------------------------------
  // AkaunKemas SaaS route protection
  // -----------------------------------------------------------------------
  const akaunkemasAuthPaths = ["/app/akaunkemas/login", "/app/akaunkemas/register"];
  const isAkaunKemasAuthPage = akaunkemasAuthPaths.some(
    (p) => pathname === p || pathname.startsWith(p + "/")
  );

  if (
    pathname.startsWith("/app/akaunkemas") &&
    !isAkaunKemasAuthPage &&
    process.env.NEXT_PUBLIC_DEMO_MODE !== "true"
  ) {
    const cookie = request.cookies.get("ak_session");
    if (cookie) {
      const session = await decrypt(cookie.value);
      if (!session || new Date() > session.expiresAt) {
        const loginUrl = new URL("/app/akaunkemas/login", request.nextUrl);
        loginUrl.searchParams.set("redirect", pathname);
        return NextResponse.redirect(loginUrl);
      }
    } else {
      const loginUrl = new URL("/app/akaunkemas/login", request.nextUrl);
      loginUrl.searchParams.set("redirect", pathname);
      return NextResponse.redirect(loginUrl);
    }
  }

  // -----------------------------------------------------------------------
  // Tool API rate limiting
  // -----------------------------------------------------------------------

  // Only apply to tool API routes
  if (pathname.startsWith("/api/tools/")) {
    if (isLocalPlaywrightRequest(request)) {
      return NextResponse.next();
    }

    const { ok, remaining } = getRateLimit(ip);

    if (!ok) {
      return NextResponse.json(
        { error: "Too many requests. Please slow down and try again later." },
        { 
          status: 429,
          headers: {
            "Retry-After": "60",
            "X-RateLimit-Limit": String(LIMITS.RATE_LIMIT_MAX),
            "X-RateLimit-Remaining": "0",
          }
        }
      );
    }

    const response = NextResponse.next();
    response.headers.set("X-RateLimit-Remaining", String(remaining));
    return response;
  }

  return NextResponse.next();
}

export const config = {
  matcher: ["/api/tools/:path*", "/app/akaunkemas/:path*"],
};

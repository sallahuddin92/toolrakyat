"use client";

import { useEffect } from "react";

const KEY = "toolrakyat:recent_tools";
const MAX = 12;

export function RecentlyUsedTracker({ toolId }: { toolId: string }) {
  useEffect(() => {
    try {
      const raw = window.localStorage.getItem(KEY);
      const current = raw ? (JSON.parse(raw) as unknown) : [];
      const list = Array.isArray(current) ? current.filter((x) => typeof x === "string") : [];

      const next = [toolId, ...list.filter((x) => x !== toolId)].slice(0, MAX);
      window.localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      // Ignore localStorage failures (private mode, quotas, etc).
    }
  }, [toolId]);

  return null;
}


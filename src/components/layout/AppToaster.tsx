"use client";

import { Toaster } from "react-hot-toast";

export function AppToaster() {
  // Central place for toast rendering. SmartPDF UI relies on react-hot-toast.
  return (
    <Toaster
      position="bottom-right"
      toastOptions={{
        duration: 4000,
        style: {
          background: "#0b1220",
          color: "#e2e8f0",
          border: "1px solid rgba(148, 163, 184, 0.25)",
        },
      }}
    />
  );
}


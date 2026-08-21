"use client";

import { SmartPdfEditor } from "@/components/tools/implementations/pdf/SmartPdfEditor";

export default function SmartPdfAppPage() {
  return (
    <main className="h-full w-full flex flex-col overflow-hidden bg-slate-100">
      <SmartPdfEditor />
    </main>
  );
}

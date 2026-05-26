import type { Metadata } from "next";

export const metadata: Metadata = {
  title: { template: "%s | AkaunKemas", default: "AkaunKemas" },
  description: "Malaysian SME bookkeeping — SaaS",
};

export default function AppLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="min-h-screen bg-slate-50">
      {children}
    </div>
  );
}

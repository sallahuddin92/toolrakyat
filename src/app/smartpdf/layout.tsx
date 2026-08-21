import type { Metadata } from "next";

export const metadata: Metadata = {
  title: "SmartPDF — Advanced Browser PDF Editor",
  description:
    "Desktop-class local PDF editor powered by StarPDF WebAssembly. 100% private, direct object manipulation, AcroForm filling, and page operations.",
};

export default function SmartPdfLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="h-screen w-screen overflow-hidden flex flex-col bg-slate-100 select-none">
      {children}
    </div>
  );
}

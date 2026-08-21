import Link from "next/link";
import { ArrowRight, ShieldCheck, Cpu, Layers, MousePointerClick, FileSpreadsheet } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";

export function SmartPdfLaunchCard() {
  return (
    <div className="w-full max-w-3xl mx-auto py-4 sm:py-8 space-y-6">
      <Card className="rounded-3xl border border-slate-200 bg-white/90 shadow-sm overflow-hidden p-6 sm:p-10">
        <CardContent className="p-0 space-y-6">
          <div className="flex flex-col sm:flex-row items-start sm:items-center justify-between gap-4">
            <div>
              <div className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-semibold bg-sky-50 text-sky-700 border border-sky-200/60 mb-2">
                <Cpu className="size-3.5" />
                <span>Powered by StarPDF Rust/WASM Engine</span>
              </div>
              <h2 className="text-2xl sm:text-3xl font-bold text-slate-900 tracking-tight">
                SmartPDF
              </h2>
              <p className="text-slate-600 text-sm sm:text-base mt-1 max-w-xl">
                Advanced, desktop-class browser PDF application with direct object manipulation, AcroForm filling, and lossless page operations.
              </p>
            </div>

            <Button asChild size="lg" className="rounded-2xl px-6 bg-sky-600 hover:bg-sky-700 shadow-sm text-white shrink-0">
              <Link href="/smartpdf" data-testid="open-smartpdf-btn" aria-label="Open SmartPDF">
                Open SmartPDF
                <ArrowRight className="size-4 ml-2" />
              </Link>
            </Button>
          </div>

          <div className="grid grid-cols-1 sm:grid-cols-2 gap-3 pt-2">
            <div className="p-3.5 rounded-2xl bg-slate-50 border border-slate-200/70 flex items-start gap-3">
              <div className="p-2 rounded-xl bg-white border border-slate-200 text-sky-600 shrink-0">
                <MousePointerClick className="size-4" />
              </div>
              <div className="text-xs">
                <div className="font-semibold text-slate-900">Direct Object Manipulation</div>
                <div className="text-slate-500 mt-0.5">Click text, images, shapes, and forms directly on the canvas to edit.</div>
              </div>
            </div>

            <div className="p-3.5 rounded-2xl bg-slate-50 border border-slate-200/70 flex items-start gap-3">
              <div className="p-2 rounded-xl bg-white border border-slate-200 text-sky-600 shrink-0">
                <FileSpreadsheet className="size-4" />
              </div>
              <div className="text-xs">
                <div className="font-semibold text-slate-900">Interactive AcroForms</div>
                <div className="text-slate-500 mt-0.5">Fill text fields, toggle checkboxes, radios, and dropdowns in-place.</div>
              </div>
            </div>

            <div className="p-3.5 rounded-2xl bg-slate-50 border border-slate-200/70 flex items-start gap-3">
              <div className="p-2 rounded-xl bg-white border border-slate-200 text-sky-600 shrink-0">
                <Layers className="size-4" />
              </div>
              <div className="text-xs">
                <div className="font-semibold text-slate-900">Lossless Page Operations</div>
                <div className="text-slate-500 mt-0.5">Reorder, duplicate, insert blank, extract, and merge PDFs instantly.</div>
              </div>
            </div>

            <div className="p-3.5 rounded-2xl bg-slate-50 border border-slate-200/70 flex items-start gap-3">
              <div className="p-2 rounded-xl bg-white border border-slate-200 text-emerald-600 shrink-0">
                <ShieldCheck className="size-4" />
              </div>
              <div className="text-xs">
                <div className="font-semibold text-slate-900">100% Local-First Privacy</div>
                <div className="text-slate-500 mt-0.5">Processed entirely in your browser. Zero bytes uploaded to any server.</div>
              </div>
            </div>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

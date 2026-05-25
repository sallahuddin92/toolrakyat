import { Suspense } from "react";
import type { Metadata } from "next";

import { Container } from "@/components/layout/Container";
import { ToolDirectory } from "@/components/tools/ToolDirectory";

export const metadata: Metadata = {
  title: "All Tools",
  description:
    "Browse all ToolRakyat tools: PDF, image, compression, converter, text, business, calculators, developer tools, QR, and more.",
};

export default function ToolsIndexPage() {
  return (
    <div className="bg-gradient-to-b from-white to-slate-50">
      <Container className="py-10 sm:py-14">
        <h1 className="text-3xl font-semibold tracking-tight text-slate-900 sm:text-4xl">
          Tools
        </h1>
        <p className="mt-3 max-w-2xl text-base leading-7 text-slate-600">
          Search and filter tools across PDF, images, business documents, text,
          developer utilities, and more.
        </p>

        <div className="mt-8">
          <Suspense
            fallback={
              <div className="rounded-2xl border bg-white p-8 text-center text-slate-600">
                Loading tools...
              </div>
            }
          >
            <ToolDirectory />
          </Suspense>
        </div>
      </Container>
    </div>
  );
}


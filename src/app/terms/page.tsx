import type { Metadata } from "next";

import { Container } from "@/components/layout/Container";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export const metadata: Metadata = {
  title: "Terms of Use",
  description:
    "ToolRakyat terms of use and disclaimers for file tools, generators, and privacy-first processing.",
};

export default function TermsPage() {
  return (
    <div className="bg-gradient-to-b from-white to-slate-50">
      <Container className="py-10 sm:py-14">
        <h1 className="text-3xl font-semibold tracking-tight text-slate-900 sm:text-4xl">
          Terms of Use
        </h1>
        <p className="mt-3 max-w-3xl text-base leading-7 text-slate-600">
          These terms are a simple starting point for the MVP. We’ll harden the
          legal text before production release.
        </p>

        <div className="mt-8 grid gap-4">
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Use responsibly</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              You are responsible for the files you upload and the outputs you
              generate. Do not upload confidential content unless you are
              comfortable with online processing.
            </CardContent>
          </Card>
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">No warranties</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              Tools are provided as-is. Some conversions/compression may have
              technical limitations; we’ll state them clearly on each tool page.
            </CardContent>
          </Card>
        </div>
      </Container>
    </div>
  );
}


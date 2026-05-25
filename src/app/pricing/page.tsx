import type { Metadata } from "next";
import Link from "next/link";

import { Container } from "@/components/layout/Container";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export const metadata: Metadata = {
  title: "Pricing",
  description:
    "ToolRakyat pricing: core tools are free. Some advanced tools may require provider setup later.",
};

export default function PricingPage() {
  return (
    <div className="bg-gradient-to-b from-white to-slate-50">
      <Container className="py-10 sm:py-14">
        <h1 className="text-3xl font-semibold tracking-tight text-slate-900 sm:text-4xl">
          Pricing
        </h1>
        <p className="mt-3 max-w-3xl text-base leading-7 text-slate-600">
          ToolRakyat focuses on shipping essential tools for free.
        </p>

        <div className="mt-8 grid gap-4 lg:grid-cols-2">
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Free</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              <ul className="space-y-2">
                <li>Core PDF, image, text, QR, and developer tools</li>
                <li>Privacy-first temporary processing</li>
                <li>No account required</li>
              </ul>
              <div className="mt-4">
                <Button asChild className="rounded-2xl">
                  <Link href="/tools">Browse tools</Link>
                </Button>
              </div>
            </CardContent>
          </Card>

          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Provider-required (future)</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              Some AI tools may require you to configure an API provider key.
              ToolRakyat will never fake outputs when keys are missing.
              <div className="mt-4">
                <Button asChild variant="secondary" className="rounded-2xl">
                  <Link href="/tools/ai">See AI tools</Link>
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      </Container>
    </div>
  );
}


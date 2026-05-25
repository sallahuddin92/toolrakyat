import type { Metadata } from "next";

import { Container } from "@/components/layout/Container";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";

export const metadata: Metadata = {
  title: "Privacy Policy",
  description:
    "ToolRakyat privacy policy: temporary file processing, no permanent storage by default, and clear handling of external providers.",
};

export default function PrivacyPage() {
  return (
    <div className="bg-gradient-to-b from-white to-slate-50">
      <Container className="py-10 sm:py-14">
        <h1 className="text-3xl font-semibold tracking-tight text-slate-900 sm:text-4xl">
          Privacy Policy
        </h1>
        <p className="mt-3 max-w-3xl text-base leading-7 text-slate-600">
          ToolRakyat is built to be privacy-first. We process files temporarily
          for the tool you choose and do not permanently store uploads by
          default.
        </p>

        <div className="mt-8 grid gap-4">
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Temporary processing</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              Uploaded files are processed for the requested tool and then
              cleaned up. Processed outputs are returned to you for download.
            </CardContent>
          </Card>
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">No hidden third parties</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              We do not send user files to third-party services unless a tool
              clearly states it (for example, future AI tools requiring a
              provider key).
            </CardContent>
          </Card>
          <Card className="rounded-2xl">
            <CardHeader>
              <CardTitle className="text-base">Limitations</CardTitle>
            </CardHeader>
            <CardContent className="text-sm text-slate-600">
              Some tools require external binaries or providers (OCR, background
              removal, some conversions). Those tools will be labeled honestly
              and won’t pretend to work without setup.
              <Separator className="my-4" />
              This policy will be expanded as features ship (accounts/history,
              analytics choices, etc.).
            </CardContent>
          </Card>
        </div>
      </Container>
    </div>
  );
}


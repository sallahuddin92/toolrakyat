import type React from "react";

import { Container } from "@/components/layout/Container";
import { PrivacyNotice } from "@/components/tools/PrivacyNotice";
import { RelatedTools } from "@/components/tools/RelatedTools";
import { Separator } from "@/components/ui/separator";
import type { ToolCategoryId } from "@/lib/tools/types";

export function ToolPageShell({
  title,
  description,
  privacyNote,
  acceptedFileTypes,
  maxFileSizeMB,
  categoryId,
  toolId,
  main,
  sidebar,
  children,
}: {
  title: string;
  description: string;
  privacyNote?: string;
  acceptedFileTypes?: string[];
  maxFileSizeMB?: number;
  categoryId?: ToolCategoryId;
  toolId?: string;
  main?: React.ReactNode;
  sidebar?: React.ReactNode;
  children?: React.ReactNode;
}) {
  const mainNode = main ?? children;
  const hasAside = Boolean(sidebar) || Boolean(privacyNote);

  return (
    <main className="bg-gradient-to-b from-white to-slate-50" data-testid="tool-page-main">
      <Container className="py-10 sm:py-14">
        <div className="max-w-3xl">
          <h1 className="text-3xl font-semibold tracking-tight text-slate-900 sm:text-4xl">
            {title}
          </h1>
          <p className="mt-3 text-base leading-7 text-slate-600">
            {description}
          </p>
        </div>

        <Separator className="my-8" />

        {mainNode && hasAside ? (
          <div className="grid gap-6 lg:grid-cols-3">
            <div className="space-y-6 lg:col-span-2">{mainNode}</div>
            <div className="space-y-6">
              {sidebar}
              {privacyNote ? (
                <PrivacyNotice
                  note={privacyNote}
                  acceptedFileTypes={acceptedFileTypes}
                  maxFileSizeMB={maxFileSizeMB}
                />
              ) : null}
            </div>
          </div>
        ) : mainNode ? (
          <div className="space-y-6">{mainNode}</div>
        ) : null}

        {categoryId && toolId ? (
          <RelatedTools categoryId={categoryId} currentToolId={toolId} />
        ) : null}
      </Container>
    </main>
  );
}

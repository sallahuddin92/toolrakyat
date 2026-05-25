import Link from "next/link";

import { Container } from "@/components/layout/Container";
import { HomeHero } from "@/components/marketing/HomeHero";
import { ToolCard } from "@/components/tools/ToolCard";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { getPopularTools, tools } from "@/lib/tools/registry";
import { TOOL_CATEGORIES } from "@/lib/tools/types";

export default function HomePage() {
  const popular = getPopularTools(9);
  const byCategory = TOOL_CATEGORIES.map((c) => ({
    ...c,
    count: tools.filter((t) => t.categoryId === c.id).length,
  }));

  return (
    <div>
      <HomeHero />

      <section className="bg-white">
        <Container className="py-12">
          <div className="flex items-end justify-between gap-4">
            <div>
              <h2 className="text-2xl font-semibold tracking-tight text-slate-900">
                Popular tools
              </h2>
              <p className="mt-2 text-sm text-slate-600">
                Fast essentials people use every day.
              </p>
            </div>
            <Button asChild variant="secondary" className="rounded-2xl">
              <Link href="/tools">View all</Link>
            </Button>
          </div>

          <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {popular.map((tool) => (
              <ToolCard key={tool.id} tool={tool} />
            ))}
          </div>
        </Container>
      </section>

      <section className="bg-gradient-to-b from-white to-slate-50">
        <Container className="py-12">
          <div>
            <h2 className="text-2xl font-semibold tracking-tight text-slate-900">
              Tool categories
            </h2>
            <p className="mt-2 text-sm text-slate-600">
              Everything organized for quick access.
            </p>
          </div>

          <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {byCategory.map((c) => (
              <Card key={c.id} className="rounded-2xl">
                <CardHeader>
                  <CardTitle className="text-base">{c.label}</CardTitle>
                </CardHeader>
                <CardContent className="space-y-3 text-sm text-slate-600">
                  <div>{c.count} tools</div>
                  <Button asChild className="rounded-2xl">
                    <Link href={`/tools/${c.id}`}>Browse {c.label}</Link>
                  </Button>
                </CardContent>
              </Card>
            ))}
          </div>
        </Container>
      </section>

      <section className="bg-white">
        <Container className="py-12">
          <div className="grid gap-6 lg:grid-cols-2">
            <Card className="rounded-2xl">
              <CardHeader>
                <CardTitle className="text-base">Privacy-first promise</CardTitle>
              </CardHeader>
              <CardContent className="text-sm text-slate-600">
                <p>
                  ToolRakyat processes files temporarily for the tool you choose.
                  We do not permanently store your files by default, and tools
                  should never send your files to third parties unless clearly
                  stated.
                </p>
                <Separator className="my-4" />
                <div className="flex gap-2">
                  <Button asChild variant="secondary" className="rounded-2xl">
                    <Link href="/privacy">Read privacy policy</Link>
                  </Button>
                </div>
              </CardContent>
            </Card>

            <Card className="rounded-2xl">
              <CardHeader>
                <CardTitle className="text-base">Why ToolRakyat</CardTitle>
              </CardHeader>
              <CardContent className="text-sm text-slate-600">
                <ul className="space-y-2">
                  <li>One website for daily PDF, image, and business tasks</li>
                  <li>Clean modern UI that stays out of your way</li>
                  <li>Built for students, SMEs, and freelancers</li>
                  <li>Honest limitations: no fake “AI results”</li>
                </ul>
                <Separator className="my-4" />
                <Button asChild className="rounded-2xl">
                  <Link href="/tools">Start with tools</Link>
                </Button>
              </CardContent>
            </Card>
          </div>
        </Container>
      </section>

      <section className="bg-gradient-to-b from-white to-slate-50">
        <Container className="py-12">
          <div className="max-w-3xl">
            <h2 className="text-2xl font-semibold tracking-tight text-slate-900">
              FAQ
            </h2>
            <p className="mt-2 text-sm text-slate-600">
              Quick answers about privacy and usage.
            </p>
          </div>

          <div className="mt-6 grid gap-4 lg:grid-cols-2">
            <Card className="rounded-2xl">
              <CardHeader>
                <CardTitle className="text-base">
                  Do you store my uploaded files?
                </CardTitle>
              </CardHeader>
              <CardContent className="text-sm text-slate-600">
                By default, no. Tools use temporary processing and clean up
                after results are generated.
              </CardContent>
            </Card>
            <Card className="rounded-2xl">
              <CardHeader>
                <CardTitle className="text-base">
                  Are the tools free?
                </CardTitle>
              </CardHeader>
              <CardContent className="text-sm text-slate-600">
                Core practical tools are free. Some advanced tools may require
                provider setup (e.g., AI features) and will be clearly labeled.
              </CardContent>
            </Card>
          </div>
        </Container>
      </section>
    </div>
  );
}


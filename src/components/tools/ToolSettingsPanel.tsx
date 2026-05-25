import type React from "react";

import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function ToolSettingsPanel({
  title = "Settings",
  children,
}: {
  title?: string;
  children: React.ReactNode;
}) {
  return (
    <Card className="rounded-2xl">
      <CardHeader>
        <CardTitle className="text-base">{title}</CardTitle>
      </CardHeader>
      <CardContent className="space-y-4">{children}</CardContent>
    </Card>
  );
}


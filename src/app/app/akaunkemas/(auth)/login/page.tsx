"use client";

import { useState } from "react";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import Link from "next/link";
import { useRouter } from "next/navigation";

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

const loginSchema = z.object({
  email: z.email(),
  password: z.string().min(8),
});

type LoginFormData = z.infer<typeof loginSchema>;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export default function LoginPage() {
  const router = useRouter();
  const [formData, setFormData] = useState<LoginFormData>({
    email: "",
    password: "",
  });
  const [errors, setErrors] = useState<Partial<Record<keyof LoginFormData, string>>>({});
  const [submitting, setSubmitting] = useState(false);

  function handleChange(field: keyof LoginFormData) {
    return (e: React.ChangeEvent<HTMLInputElement>) => {
      setFormData((prev) => ({ ...prev, [field]: e.target.value }));
      // Clear the error for this field on change
      if (errors[field]) {
        setErrors((prev) => ({ ...prev, [field]: undefined }));
      }
    };
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);

    const result = loginSchema.safeParse(formData);
    if (!result.success) {
      const fieldErrors: Partial<Record<keyof LoginFormData, string>> = {};
      for (const issue of result.error.issues) {
        const field = issue.path[0] as keyof LoginFormData;
        if (!fieldErrors[field]) {
          fieldErrors[field] = issue.message;
        }
      }
      setErrors(fieldErrors);
      setSubmitting(false);
      return;
    }

    // DEV-ONLY: No real authentication backend exists yet.
    // In production this would call an API route that creates a server-side session.
    console.warn(
      "[DEV-ONLY] Login submitted with email:",
      result.data.email,
      "— no real auth check performed."
    );

    // Simulate a brief network delay for realistic UX.
    await new Promise((resolve) => setTimeout(resolve, 800));

    // DEV-ONLY: Redirect to the main app without real session creation.
    router.push("/app/akaunkemas");
  }

  return (
    <div className="relative">
      {/* DEV-ONLY badge */}
      <Badge
        variant="secondary"
        className="absolute -top-3 left-1/2 -translate-x-1/2 border-yellow-600/30 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400 text-xs"
      >
        Dev-only demo login — no real authentication
      </Badge>

      <Card size="sm" className="w-full max-w-md mx-auto mt-6">
        <CardHeader className="text-center">
          <CardTitle>Sign in to AkaunKemas</CardTitle>
          <CardDescription>
            Enter your email and password to access your dashboard.
          </CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="flex flex-col gap-4">
            <div className="flex flex-col gap-1.5">
              <Label htmlFor="email">Email</Label>
              <Input
                id="email"
                type="email"
                placeholder="you@example.com"
                value={formData.email}
                onChange={handleChange("email")}
                aria-invalid={!!errors.email}
              />
              {errors.email && (
                <p className="text-xs text-destructive">{errors.email}</p>
              )}
            </div>

            <div className="flex flex-col gap-1.5">
              <Label htmlFor="password">Password</Label>
              <Input
                id="password"
                type="password"
                placeholder="Minimum 8 characters"
                value={formData.password}
                onChange={handleChange("password")}
                aria-invalid={!!errors.password}
              />
              {errors.password && (
                <p className="text-xs text-destructive">{errors.password}</p>
              )}
            </div>

            <Button type="submit" className="w-full mt-2" disabled={submitting}>
              {submitting ? "Signing in..." : "Sign in"}
            </Button>
          </form>

          <p className="text-center text-sm text-muted-foreground mt-4">
            Don&apos;t have an account?{" "}
            <Link href="/app/akaunkemas/register" className="text-primary underline underline-offset-4 hover:no-underline">
              Create one
            </Link>
          </p>
        </CardContent>
      </Card>
    </div>
  );
}

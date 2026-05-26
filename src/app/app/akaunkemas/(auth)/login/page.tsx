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
import { loginAction } from "./actions";

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
    email: process.env.NEXT_PUBLIC_DEMO_MODE === "true" ? "demo@akaunkemas.my" : "",
    password: "",
  });
  const [errors, setErrors] = useState<Partial<Record<keyof LoginFormData, string>>>({});
  const [serverError, setServerError] = useState<string | null>(null);
  const [submitting, setSubmitting] = useState(false);

  const isDemoMode = process.env.NEXT_PUBLIC_DEMO_MODE === "true";

  function handleChange(field: keyof LoginFormData) {
    return (e: React.ChangeEvent<HTMLInputElement>) => {
      setFormData((prev) => ({ ...prev, [field]: e.target.value }));
      if (errors[field]) {
        setErrors((prev) => ({ ...prev, [field]: undefined }));
      }
      if (serverError) {
        setServerError(null);
      }
    };
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSubmitting(true);
    setServerError(null);

    // Client-side validation
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

    // Submit via Server Action
    try {
      const fd = new FormData();
      fd.set("email", result.data.email);
      fd.set("password", result.data.password);

      const actionResult = await loginAction(fd);
      if (actionResult && actionResult.error) {
        setServerError(actionResult.error);
        setSubmitting(false);
      }
      // If no error returned, the Server Action performed a redirect
    } catch {
      setServerError("An unexpected error occurred. Please try again.");
      setSubmitting(false);
    }
  }

  // DEV-ONLY: quick demo login
  async function handleDemoLogin() {
    setSubmitting(true);
    setServerError(null);

    const fd = new FormData();
    fd.set("email", "demo@akaunkemas.my");
    fd.set("password", "demo1234");

    try {
      const actionResult = await loginAction(fd);
      if (actionResult && actionResult.error) {
        setServerError(actionResult.error);
        setSubmitting(false);
      }
    } catch {
      setServerError("An unexpected error occurred. Please try again.");
      setSubmitting(false);
    }
  }

  return (
    <div className="relative">
      {/* DEV-ONLY badge */}
      {isDemoMode && (
        <Badge
          variant="secondary"
          className="absolute -top-3 left-1/2 -translate-x-1/2 border-yellow-600/30 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400 text-xs"
        >
          Dev-only demo login
        </Badge>
      )}

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

            {serverError && (
              <p className="text-sm text-destructive bg-destructive/10 rounded-md p-2">
                {serverError}
              </p>
            )}

            <Button type="submit" className="w-full mt-2" disabled={submitting}>
              {submitting ? "Signing in..." : "Sign in"}
            </Button>
          </form>

          {isDemoMode && (
            <div className="mt-4 rounded-md border border-yellow-600/20 bg-yellow-500/5 p-3">
              <p className="text-xs text-muted-foreground mb-2">
                Demo credentials: <code className="bg-muted px-1 py-0.5 rounded">demo@akaunkemas.my</code> / <code className="bg-muted px-1 py-0.5 rounded">demo1234</code>
              </p>
              <Button
                variant="outline"
                size="sm"
                className="w-full"
                disabled={submitting}
                onClick={handleDemoLogin}
              >
                Quick demo login
              </Button>
            </div>
          )}

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

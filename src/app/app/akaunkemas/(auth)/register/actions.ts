"use server";

import { registerUser, loginUser } from "@/lib/auth/auth-service";
import { redirect } from "next/navigation";

export async function registerAction(formData: FormData) {
  const name = formData.get("name") as string;
  const email = formData.get("email") as string;
  const password = formData.get("password") as string;

  const result = await registerUser(email, name, password);

  if (!result.success) {
    return { error: result.error };
  }

  // Auto-login after successful registration
  const loginResult = await loginUser(email, password);

  if (!loginResult.success) {
    // Registration succeeded but auto-login failed — redirect to login page
    redirect("/app/akaunkemas/login");
  }

  redirect("/app/akaunkemas");
}

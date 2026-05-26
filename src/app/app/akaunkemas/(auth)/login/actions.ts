"use server";

import { loginUser } from "@/lib/auth/auth-service";
import { redirect } from "next/navigation";

export async function loginAction(formData: FormData) {
  const email = formData.get("email") as string;
  const password = formData.get("password") as string;

  const result = await loginUser(email, password);

  if (!result.success) {
    return { error: result.error };
  }

  redirect("/app/akaunkemas");
}

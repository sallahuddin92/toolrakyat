import { logoutUser } from "@/lib/auth/auth-service";
import { redirect } from "next/navigation";

export async function GET() {
  await logoutUser();
  redirect("/app/akaunkemas/login");
}

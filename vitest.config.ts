import { defineConfig } from "vitest/config";
import path from "path";

export default defineConfig({
  resolve: {
    alias: {
      "@/lib/pdf": path.resolve(__dirname, "./apps/smartpdf-web/src/lib/pdf"),
      "@/components/smartpdf": path.resolve(__dirname, "./apps/smartpdf-web/src/components"),
      "@smartpdf": path.resolve(__dirname, "./apps/smartpdf-web/src"),
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});


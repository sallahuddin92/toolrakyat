import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  serverExternalPackages: ["@napi-rs/canvas"],
  webpack: (config) => {
    config.watchOptions = {
      ...config.watchOptions,
      ignored: ["**/test-results/**", "**/e2e/**", "**/node_modules/**"],
    };
    return config;
  },
};

export default nextConfig;

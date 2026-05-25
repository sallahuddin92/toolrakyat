import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { SiteFooter } from "@/components/layout/SiteFooter";
import { SiteHeader } from "@/components/layout/SiteHeader";
import { AppToaster } from "@/components/layout/AppToaster";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: {
    default: "ToolRakyat",
    template: "%s | ToolRakyat",
  },
  description: "Free practical productivity tools for everyone.",
  applicationName: "ToolRakyat",
  keywords: [
    "PDF tools",
    "image tools",
    "compression tools",
    "converter tools",
    "text tools",
    "invoice generator",
    "QR code generator",
    "developer tools",
    "calculator tools",
  ],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="min-h-full flex flex-col bg-background text-foreground">
        <SiteHeader />
        <div className="flex-1">{children}</div>
        <SiteFooter />
        <AppToaster />
      </body>
    </html>
  );
}

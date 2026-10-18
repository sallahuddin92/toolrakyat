import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";
import { Toaster } from "react-hot-toast";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "SmartPDF — Precision Document Engineering",
  description:
    "Desktop-class professional PDF application powered by StarPDF Rust engine. 100% private, local-first editing, AcroForm filling, and page operations.",
  applicationName: "SmartPDF",
  keywords: [
    "PDF editor",
    "SmartPDF",
    "StarPDF",
    "Rust PDF",
    "local-first",
    "AcroForm",
    "multilingual PDF",
  ],
};

export default function SmartPdfRootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="h-full w-full overflow-hidden flex flex-col bg-slate-100 text-slate-900 select-none">
        {children}
        <Toaster position="bottom-right" />
      </body>
    </html>
  );
}

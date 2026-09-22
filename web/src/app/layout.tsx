import type { Metadata } from "next";
import Link from "next/link";
import { Geist, Geist_Mono } from "next/font/google";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "3D Patches",
  description:
    "A weekly 3D take on the Patches puzzle, rendered with Rust and WebAssembly.",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <body className="flex min-h-full flex-col font-sans">
        <header className="mx-auto flex w-full max-w-3xl items-baseline justify-between px-6 py-8">
          <Link href="/" className="text-2xl font-bold tracking-tight">
            3D Patches
          </Link>
          <nav className="flex gap-6 text-sm">
            <Link href="/">Game</Link>
            <Link href="/answer">Answer</Link>
            <a href="https://github.com/kaedeng">GitHub</a>
          </nav>
        </header>
        <main className="mx-auto w-full max-w-3xl flex-1 px-6 pb-16">
          {children}
        </main>
        <footer className="mx-auto w-full max-w-3xl px-6 py-8 text-sm text-zinc-500">
          Made by Kae. Rust + WebAssembly + Next.js. New puzzle every Monday.
        </footer>
      </body>
    </html>
  );
}

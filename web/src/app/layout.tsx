import type { Metadata } from "next";
import Link from "next/link";
import { Geist, Geist_Mono } from "next/font/google";
import { ProfileButton } from "@/components/ProfileButton";
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
      <body className="flex min-h-full flex-col font-sans text-lg leading-relaxed">
        <header className="mx-auto flex w-full max-w-3xl items-center justify-between px-6 py-8">
          <div className="flex items-center gap-3">
            <ProfileButton />
            <Link href="/" className="font-semibold tracking-tight">
              3D Patches
            </Link>
          </div>
          <nav className="flex gap-4 text-sm text-zinc-400 sm:gap-6">
            <Link href="/" className="hover:text-white">
              Game
            </Link>
            <Link href="/answer" className="hover:text-white">
              Answer
            </Link>
            <a href="https://github.com/kaedeng" className="hover:text-white">
              GitHub
            </a>
          </nav>
        </header>
        <main className="mx-auto w-full max-w-3xl flex-1 px-6 pt-8 pb-24">
          {children}
        </main>
        <footer className="mx-auto w-full max-w-3xl px-6 py-8 text-sm text-zinc-400">
          Made by Kae. Rust + WebAssembly + Next.js. New puzzle every Monday.
        </footer>
      </body>
    </html>
  );
}

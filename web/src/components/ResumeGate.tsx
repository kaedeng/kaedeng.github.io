"use client";

import Link from "next/link";
import type { ReactNode } from "react";
import { useResumeUnlocked } from "@/lib/unlock";

/** Shows `children` once the player has solved a puzzle, and a way back to it until then. */
export function ResumeGate({ children }: { children: ReactNode }) {
  const unlocked = useResumeUnlocked();
  if (unlocked === null) return null;
  if (unlocked) return children;
  return (
    <>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        Locked
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Solve this week&apos;s puzzle to open the résumé.
      </p>
      <Link
        href="/"
        className="mt-8 inline-block rounded-md bg-white px-4 py-2 text-sm font-medium text-black hover:bg-zinc-200"
      >
        Play the puzzle
      </Link>
    </>
  );
}

"use client";

import Link from "next/link";
import { useResumeUnlocked } from "@/lib/unlock";

const CIRCLE = "grid size-8 place-items-center rounded-full border";

function PersonIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      className="size-4"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      aria-hidden
    >
      <circle cx="12" cy="8" r="4" />
      <path d="M20 21a8 8 0 0 0-16 0" />
    </svg>
  );
}

/** Greyed out until the player solves a puzzle, then a link to the résumé. */
export function ProfileButton() {
  const unlocked = useResumeUnlocked();
  if (!unlocked) {
    return (
      <span
        role="link"
        aria-disabled="true"
        aria-label="Résumé, locked until you solve the puzzle"
        title="Solve the puzzle to unlock"
        className={`${CIRCLE} cursor-not-allowed border-white/10 text-zinc-600`}
      >
        <PersonIcon />
      </span>
    );
  }
  return (
    <Link
      href="/resume"
      aria-label="Résumé"
      title="Résumé"
      className={`${CIRCLE} border-white/25 text-white hover:bg-white/10`}
    >
      <PersonIcon />
    </Link>
  );
}

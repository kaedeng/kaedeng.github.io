"use client";

import { useState } from "react";
import type { Puzzle } from "patches-board";
import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";

/** The puzzle's solution behind a disclosure that starts closed. */
export function Solution({ puzzle }: { puzzle: Puzzle }) {
  const [showSolution, setShowSolution] = useState(false);
  return (
    <details
      className="mt-12"
      open={showSolution}
      onToggle={(e) => setShowSolution(e.currentTarget.open)}
    >
      <summary className="cursor-pointer text-sm text-zinc-400 hover:text-white">
        {showSolution ? "Hide solution" : "Show solution"}
      </summary>
      {showSolution && (
        <div className="mt-6">
          <Cube puzzle={puzzle} mode="answer" />
          <div className="mt-12">
            <Layers puzzle={puzzle} showSolution />
          </div>
        </div>
      )}
    </details>
  );
}

"use client";

import { useState } from "react";
import { generatePuzzle, type Puzzle } from "patches-board";
import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";
import { Tutorial } from "@/components/Tutorial";

type Mode = "weekly" | "random";

const MODES: [Mode, string][] = [
  ["weekly", "Weekly"],
  ["random", "Random"],
];

/** Six hex digits: the generator's seed, and short enough to read as the puzzle's id. */
function randomSeed(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(3));
  return Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
}

/**
 * The game page's puzzle: this week's, or a random one generated in the browser with its
 * solution behind a disclosure. Both boards stay mounted, so switching keeps progress.
 */
export function GameModes({ weekly }: { weekly: Puzzle }) {
  const [mode, setMode] = useState<Mode>("weekly");
  const [random, setRandom] = useState<Puzzle | null>(null);
  const [showSolution, setShowSolution] = useState(false);
  const puzzle = mode === "weekly" ? weekly : random;

  const newRandom = async () => {
    setShowSolution(false);
    setRandom(await generatePuzzle(randomSeed()));
  };
  const choose = (m: Mode) => {
    setMode(m);
    if (m === "random" && !random) void newRandom();
  };

  return (
    <>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        Puzzle {puzzle?.id ?? "…"}
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Fill the 4×4×4 cube with boxes. Every box holds exactly one clue and
        takes that clue&apos;s shape and volume.
      </p>
      <div className="mt-12 flex items-center justify-between gap-4">
        <div
          role="group"
          aria-label="Puzzle"
          className="inline-flex rounded-md border border-white/15 p-0.5 text-sm font-medium"
        >
          {MODES.map(([m, label]) => (
            <button
              key={m}
              type="button"
              aria-pressed={mode === m}
              onClick={() => choose(m)}
              className={`rounded px-3 py-1.5 ${mode === m ? "bg-white text-black" : "text-zinc-400 hover:text-white"}`}
            >
              {label}
            </button>
          ))}
        </div>
        <Tutorial />
      </div>
      <div className="mt-6" hidden={mode !== "weekly"}>
        <Cube puzzle={weekly} mode="play" />
      </div>
      {random && (
        <div className="mt-6" hidden={mode !== "random"}>
          <Cube
            key={random.id}
            puzzle={random}
            mode="play"
            solvedNote="Every cell is in exactly one box."
            actions={
              <button
                type="button"
                className="rounded-md border border-white/15 px-4 py-2 text-sm font-medium whitespace-nowrap hover:bg-white/10"
                onClick={() => void newRandom()}
              >
                New puzzle
              </button>
            }
          />
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
                <Cube puzzle={random} mode="answer" />
                <div className="mt-12">
                  <Layers puzzle={random} showSolution />
                </div>
              </div>
            )}
          </details>
        </div>
      )}
      {puzzle && (
        <details className="mt-16">
          <summary className="cursor-pointer text-sm text-zinc-400 hover:text-white">
            Clues layer by layer (layer 1 is the bottom)
          </summary>
          <div className="mt-6">
            <Layers puzzle={puzzle} showSolution={false} />
          </div>
        </details>
      )}
    </>
  );
}

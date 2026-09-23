"use client";

import { useEffect, useRef, useState } from "react";
import {
  mountBoard,
  PALETTE,
  type Board,
  type Puzzle,
  type Status,
} from "patches-board";
import { Confetti } from "@/components/Confetti";

/** When the first press on a cell happened, and when the puzzle was solved. */
type Clock = { start: number; end: number | null };

/** Solve time as m:ss. */
function formatTime(ms: number): string {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** The clue colours, for the solved message. */
const SOLVED_GRADIENT = `linear-gradient(90deg, ${PALETTE.slice(0, 6).join(", ")})`;

export function Cube({
  puzzle,
  mode,
}: {
  puzzle: Puzzle;
  mode: "play" | "answer";
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const boardRef = useRef<Board | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [clock, setClock] = useState<Clock | null>(null);
  const [now, setNow] = useState(0);

  // Ticks the visible timer once a second while it runs.
  useEffect(() => {
    if (!clock || clock.end !== null) return;
    const id = setInterval(() => setNow(performance.now()), 1000);
    return () => clearInterval(id);
  }, [clock]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let cancelled = false;
    // A functional update, so a burst of events faster than renders can't restart it.
    const startClock = () => {
      const t = performance.now();
      setClock((c) => c ?? { start: t, end: null });
      setNow((n) => n || t);
    };
    mountBoard(canvas, puzzle, {
      answer: mode === "answer",
      onStatus: (s) => {
        setStatus(s);
        const t = performance.now();
        setClock((c) =>
          c && c.end === null && s.solved ? { ...c, end: t } : c,
        );
      },
      onPlay: startClock,
    })
      .then((board) => {
        if (cancelled) board.destroy();
        else boardRef.current = board;
      })
      .catch((e: unknown) => {
        console.error(e);
        setError(String(e));
      });
    return () => {
      cancelled = true;
      boardRef.current?.destroy();
      boardRef.current = null;
    };
  }, [puzzle, mode]);

  const reset = () => {
    boardRef.current?.reset();
    setClock(null);
  };

  return (
    <div>
      {/* font-mono: the board's clue labels and mode line take the font of this box. */}
      <div className="relative aspect-square w-full overflow-hidden bg-black font-mono sm:aspect-auto sm:h-[min(80vh,720px)]">
        <canvas
          ref={canvasRef}
          className="h-full w-full cursor-grab touch-none outline-none focus-visible:ring-2 focus-visible:ring-white/40 focus-visible:ring-inset"
          tabIndex={0}
          aria-label="3D puzzle board. Arrow keys move a cursor, Shift with up and down changes layer, Space builds a box, Delete removes one."
        />
        {status === null && !error && (
          <p className="absolute inset-0 grid place-items-center font-sans text-sm text-zinc-400">
            Loading 3D board…
          </p>
        )}
        {error && (
          <p className="absolute inset-0 grid place-items-center p-6 text-center font-sans text-red-400">
            The 3D board could not start ({error}). The layer grids below still
            work.
          </p>
        )}
      </div>
      {mode === "play" && status && !status.solved && (
        <div className="mt-6 flex items-center justify-between gap-4">
          <span className="text-sm text-zinc-400">
            {`${status.boxes} of ${puzzle.clues.length} boxes placed`}
            {status.wrong > 0 && (
              <span className="text-red-400"> · {status.wrong} wrong</span>
            )}
          </span>
          <div className="flex items-center gap-4">
            <span className="font-mono text-sm text-zinc-400 tabular-nums">
              {formatTime(clock ? Math.max(now, clock.start) - clock.start : 0)}
            </span>
            <button
              type="button"
              className="rounded-md border border-white/15 px-4 py-2 text-sm font-medium hover:bg-white/10"
              onClick={() => reset()}
            >
              Reset
            </button>
          </div>
        </div>
      )}
      {mode === "play" && status?.solved && (
        <div className="mt-8" role="status">
          <Confetti />
          <p className="text-sm font-medium text-zinc-400">Solved</p>
          <p
            className="mt-2 bg-clip-text text-5xl font-semibold tracking-tighter text-transparent sm:text-6xl"
            style={{ backgroundImage: SOLVED_GRADIENT }}
          >
            {clock?.end ? `in ${formatTime(clock.end - clock.start)}` : "Nice."}
          </p>
          <p className="mt-4 text-zinc-400">
            Every cell is in exactly one box. A new puzzle arrives Monday.
          </p>
          <button
            type="button"
            className="mt-6 rounded-md bg-white px-4 py-2 text-sm font-medium text-black hover:bg-zinc-200"
            onClick={() => reset()}
          >
            Play again
          </button>
        </div>
      )}
    </div>
  );
}

"use client";

import { useEffect, useRef, useState, type PointerEvent } from "react";
import { clueText, type Puzzle } from "@/lib/puzzle";

// Built by `pnpm wasm` into public/wasm and loaded at runtime, outside the bundler.
type Wasm = typeof import("../../public/wasm/patches_wasm");
type Game = InstanceType<Wasm["Game"]>;
type Status = { boxes: number; solved: boolean };

const WASM_JS = "/wasm/patches_wasm.js";
const WASM_BIN = "/wasm/patches_wasm_bg.wasm";

let wasmReady: Promise<Wasm> | null = null;

function loadWasm(): Promise<Wasm> {
  wasmReady ??= (async () => {
    const wasm = (await import(
      /* webpackIgnore: true */ /* turbopackIgnore: true */ WASM_JS
    )) as Wasm;
    await wasm.default({ module_or_path: WASM_BIN });
    return wasm;
  })();
  return wasmReady;
}

function placeLabels(game: Game, spans: (HTMLSpanElement | null)[]) {
  const points = game.labels();
  spans.forEach((span, i) => {
    if (!span) return;
    span.style.display = "";
    span.style.transform = `translate(-50%, -50%) translate(${points[2 * i]}px, ${points[2 * i + 1]}px)`;
  });
}

export function Cube({
  puzzle,
  mode,
}: {
  puzzle: Puzzle;
  mode: "play" | "answer";
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const labelRefs = useRef<(HTMLSpanElement | null)[]>([]);
  const gameRef = useRef<Game | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let cancelled = false;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = Math.round(canvas.clientWidth * dpr);
    canvas.height = Math.round(canvas.clientHeight * dpr);
    loadWasm()
      .then((wasm) => {
        if (cancelled) return;
        const game = new wasm.Game(
          canvas,
          JSON.stringify(puzzle),
          mode === "answer",
        );
        gameRef.current = game;
        game.render();
        placeLabels(game, labelRefs.current);
        setStatus({ boxes: game.box_count(), solved: game.is_solved() });
      })
      .catch((e: unknown) => {
        console.error(e);
        setError(String(e));
      });
    return () => {
      cancelled = true;
      gameRef.current?.free();
      gameRef.current = null;
    };
  }, [puzzle, mode]);

  const refresh = (game: Game, boardChanged: boolean) => {
    game.render();
    placeLabels(game, labelRefs.current);
    if (boardChanged)
      setStatus({ boxes: game.box_count(), solved: game.is_solved() });
  };
  const point = (e: PointerEvent<HTMLCanvasElement>) =>
    [e.nativeEvent.offsetX, e.nativeEvent.offsetY] as const;

  return (
    <div>
      <div className="relative aspect-square w-full overflow-hidden rounded border border-zinc-300">
        <canvas
          ref={canvasRef}
          className="h-full w-full touch-none"
          onPointerDown={(e) => {
            const game = gameRef.current;
            if (!game) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            game.pointer_down(...point(e));
          }}
          onPointerMove={(e) => {
            const game = gameRef.current;
            if (game && game.pointer_move(...point(e))) refresh(game, false);
          }}
          onPointerUp={(e) => {
            const game = gameRef.current;
            if (game && game.pointer_up(...point(e))) refresh(game, true);
          }}
        />
        {puzzle.clues.map((clue, i) => (
          <span
            key={i}
            ref={(el) => {
              labelRefs.current[i] = el;
            }}
            className="pointer-events-none absolute top-0 left-0 rounded bg-white/85 px-1 font-mono text-sm font-bold text-zinc-900"
            style={{ display: "none" }}
          >
            {clueText(clue)}
          </span>
        ))}
        {status === null && !error && (
          <p className="absolute inset-0 grid place-items-center text-zinc-500">
            Loading 3D board…
          </p>
        )}
        {error && (
          <p className="absolute inset-0 grid place-items-center p-6 text-center text-red-700">
            The 3D board could not start ({error}). The layer grids below still
            work.
          </p>
        )}
      </div>
      {mode === "play" && status && (
        <div className="mt-4 flex items-center gap-4">
          <button
            type="button"
            className="rounded border border-zinc-400 px-3 py-1 text-sm"
            onClick={() => {
              const game = gameRef.current;
              if (!game) return;
              game.reset();
              refresh(game, true);
            }}
          >
            Reset
          </button>
          <span className="text-sm">
            {status.solved
              ? "Solved! Every cell is in exactly one box."
              : `${status.boxes} boxes placed`}
          </span>
        </div>
      )}
    </div>
  );
}

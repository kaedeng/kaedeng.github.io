"use client";

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type PointerEvent,
} from "react";
import { clueColor, clueText, type Puzzle } from "@/lib/puzzle";

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
  const rafRef = useRef(0);
  const lastRef = useRef(0);
  const [status, setStatus] = useState<Status | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Draws on animation frames until the tweens settle; no frames while idle.
  const animate = useCallback((game: Game) => {
    if (rafRef.current) return;
    const frame = (t: number) => {
      const dt = lastRef.current ? t - lastRef.current : 0;
      lastRef.current = t;
      const more = game.tick(dt);
      game.render();
      rafRef.current = more ? requestAnimationFrame(frame) : 0;
      if (!more) lastRef.current = 0;
    };
    rafRef.current = requestAnimationFrame(frame);
  }, []);

  const refresh = useCallback(
    (game: Game, boardChanged: boolean) => {
      placeLabels(game, labelRefs.current);
      if (boardChanged)
        setStatus({ boxes: game.box_count(), solved: game.is_solved() });
      animate(game);
    },
    [animate],
  );

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
        refresh(game, true);
      })
      .catch((e: unknown) => {
        console.error(e);
        setError(String(e));
      });
    return () => {
      cancelled = true;
      cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
      gameRef.current?.free();
      gameRef.current = null;
    };
  }, [puzzle, mode, refresh]);

  const point = (e: PointerEvent<HTMLCanvasElement>) =>
    [e.nativeEvent.offsetX, e.nativeEvent.offsetY] as const;

  return (
    <div>
      <div className="relative aspect-square w-full overflow-hidden bg-black sm:aspect-auto sm:h-[min(80vh,720px)]">
        <canvas
          ref={canvasRef}
          className="h-full w-full cursor-grab touch-none"
          onPointerDown={(e) => {
            const game = gameRef.current;
            if (!game) return;
            e.currentTarget.setPointerCapture(e.pointerId);
            game.pointer_down(...point(e));
            refresh(game, false);
          }}
          onPointerMove={(e) => {
            const game = gameRef.current;
            if (!game) return;
            if (game.pointer_move(...point(e), e.timeStamp))
              refresh(game, false);
            e.currentTarget.style.cursor = game.hovering() ? "pointer" : "";
          }}
          onPointerUp={(e) => {
            const game = gameRef.current;
            if (!game) return;
            refresh(game, game.pointer_up(...point(e)));
            e.currentTarget.style.cursor = game.hovering() ? "pointer" : "";
          }}
          onPointerLeave={() => {
            const game = gameRef.current;
            if (game && game.pointer_leave()) refresh(game, false);
          }}
          onPointerCancel={() => {
            const game = gameRef.current;
            if (!game) return;
            game.pointer_cancel();
            refresh(game, false);
          }}
        />
        {puzzle.clues.map((clue, i) => (
          <span
            key={i}
            ref={(el) => {
              labelRefs.current[i] = el;
            }}
            className="pointer-events-none absolute top-0 left-0 rounded-sm px-1 font-mono text-sm font-semibold text-black"
            style={{ display: "none", background: clueColor(i) }}
          >
            {clueText(clue)}
          </span>
        ))}
        {status === null && !error && (
          <p className="absolute inset-0 grid place-items-center text-sm text-zinc-400">
            Loading 3D board…
          </p>
        )}
        {error && (
          <p className="absolute inset-0 grid place-items-center p-6 text-center text-red-400">
            The 3D board could not start ({error}). The layer grids below still
            work.
          </p>
        )}
      </div>
      {mode === "play" && status && (
        <div className="mt-6 flex items-center justify-between gap-4">
          <span
            className={`text-sm ${status.solved ? "font-medium text-white" : "text-zinc-400"}`}
          >
            {status.solved
              ? "Solved! Every cell is in exactly one box."
              : `${status.boxes} of ${puzzle.clues.length} boxes placed`}
          </span>
          <button
            type="button"
            className="rounded-md border border-white/15 px-4 py-2 text-sm font-medium hover:bg-white/10"
            onClick={() => {
              const game = gameRef.current;
              if (!game) return;
              game.reset();
              refresh(game, true);
            }}
          >
            Reset
          </button>
        </div>
      )}
    </div>
  );
}

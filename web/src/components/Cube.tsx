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
type Status = { boxes: number; wrong: number; solved: boolean };
/** When the first press on a cell happened, and when the puzzle was solved. */
type Clock = { start: number; end: number | null };

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
    const [x, y] = [points[2 * i], points[2 * i + 1]];
    // NaN: the clue is in a peeled layer.
    span.style.display = Number.isNaN(x) ? "none" : "";
    span.style.transform = `translate(-50%, -50%) translate(${x}px, ${y}px)`;
  });
}

/** Solve time as m:ss. */
function formatTime(ms: number): string {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

/** Vivid versions of a few clue colours, for the solved message. */
const SOLVED_GRADIENT =
  "linear-gradient(90deg, #ff3030, #ffd630, #30ff7a, #30d0ff, #c030ff)";

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
  const [clock, setClock] = useState<Clock | null>(null);
  const [now, setNow] = useState(0);

  // Ticks the visible timer once a second while it runs.
  useEffect(() => {
    if (!clock || clock.end !== null) return;
    const id = setInterval(() => setNow(performance.now()), 1000);
    return () => clearInterval(id);
  }, [clock]);

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
      if (boardChanged) {
        const solved = game.is_solved();
        setStatus({
          boxes: game.box_count(),
          wrong: game.wrong_count(),
          solved,
        });
        const t = performance.now();
        setClock((c) => (c && c.end === null && solved ? { ...c, end: t } : c));
      }
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
    // Wheel and trackpad pinch (ctrl+wheel) zoom; at either end the page scrolls instead.
    const onWheel = (e: WheelEvent) => {
      const game = gameRef.current;
      const speed = e.ctrlKey ? 0.01 : 0.0015;
      if (!game || !game.zoom_by(-e.deltaY * speed)) return;
      e.preventDefault();
      refresh(game, false);
    };
    canvas.addEventListener("wheel", onWheel, { passive: false });
    return () => {
      cancelled = true;
      canvas.removeEventListener("wheel", onWheel);
      cancelAnimationFrame(rafRef.current);
      rafRef.current = 0;
      gameRef.current?.free();
      gameRef.current = null;
    };
  }, [puzzle, mode, refresh]);

  const reset = () => {
    const game = gameRef.current;
    if (!game) return;
    game.reset();
    setClock(null);
    refresh(game, true);
  };
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
            if (game.pointer_down(...point(e)) && !clock) {
              const t = performance.now();
              setClock({ start: t, end: null });
              setNow(t);
            }
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

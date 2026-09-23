"use client";

import {
  useEffect,
  useEffectEvent,
  useLayoutEffect,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
  type SyntheticEvent,
} from "react";
import { mountBoard, type Board, type Puzzle, type Shape } from "patches-board";
import { ShapeIcon } from "@/components/ShapeIcon";
import { canDemo, demoScript } from "@/lib/demo";
import { playDemo } from "@/lib/ghost";
import { SHAPE_NAME } from "@/lib/puzzle";
import { reached, type BoardEvent, type Goal } from "@/lib/tutorial";
import data from "@/tutorial.json";

/** Four flat layers: small enough to solve while learning the controls. */
const PRACTICE = data as Puzzle;
/** Set once the tutorial is closed, so it only opens by itself on a first visit. */
const SEEN = "patches-tutorial-seen";
/** Pause after a step's goal is met, so the player sees what they did. */
const ADVANCE_MS = 700;
/** How long the "Your turn" line stays lit after a demo. */
const NUDGE_MS = 1600;

/** `turn` is what the player does on the practice board to meet `goal`. */
type Step = { title: string; body: ReactNode; turn?: string; goal?: Goal };

const STEPS: Step[] = [
  {
    title: "Build a box",
    goal: "place",
    body: "Dragging from one cell to another builds the box between them.",
    turn: "drag from one corner of the top layer to the opposite corner.",
  },
  {
    title: "Remove it",
    goal: "remove",
    body: "Dragging from a box grows it instead of removing it.",
    turn: "click a box to remove it.",
  },
  {
    title: "Turn the cube",
    goal: "turn",
    body: "Scroll or pinch to zoom; zoom in far enough and the nearest layer peels away.",
    turn: "drag the empty space around the cube.",
  },
  {
    title: "See one layer flat",
    goal: "flat",
    body: "In 2D, ‹ › or Shift+↑/↓ change layer; click the cube again to go back to 3D.",
    turn: "click a face of the small cube in the corner.",
  },
  { title: "Read the clues", body: <Clues /> },
  {
    title: "Solve it",
    goal: "solve",
    body: "A box that breaks its clue shows as a red outline; drag from it to fix it.",
    turn: "fill the practice cube so every box holds exactly one clue.",
  },
  { title: "More ways to play", body: <Tips /> },
];

const listeners = new Set<() => void>();
// Also kept in memory, so a browser that refuses storage still stops reopening it.
let seenNow = false;

function isSeen(): boolean {
  if (seenNow) return true;
  try {
    return localStorage.getItem(SEEN) === "1";
  } catch {
    return false;
  }
}

function markSeen() {
  seenNow = true;
  try {
    localStorage.setItem(SEEN, "1");
  } catch {}
  listeners.forEach((l) => l());
}

function subscribe(listener: () => void) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/**
 * A "How to play" button, and the step-by-step tutorial it opens over the page, with a
 * practice board of its own. It also opens by itself on a first visit.
 */
export function Tutorial() {
  const dialogRef = useRef<HTMLDialogElement>(null);
  const firstVisit = useSyncExternalStore(
    subscribe,
    () => !isSeen(),
    () => false,
  );
  const [reopened, setReopened] = useState(false);
  const [step, setStep] = useState(0);
  const [done, setDone] = useState(false);
  const boxes = useRef(0);
  const board = useRef<Board | null>(null);
  const ghost = useRef<HTMLDivElement>(null);
  // Set while a demo plays: stops it.
  const stopDemo = useRef<(() => void) | null>(null);
  const [playing, setPlaying] = useState(false);
  // The step whose "Your turn" line is lit after its demo.
  const [nudged, setNudged] = useState<number | null>(null);
  const open = firstVisit || reopened;
  const { title, body, turn, goal } = STEPS[step];
  const last = step === STEPS.length - 1;

  // A layout effect, so the dialog is showing before the practice board mounts in its
  // own effect: the board sizes its drawing buffer to the canvas then.
  useLayoutEffect(() => {
    const dialog = dialogRef.current;
    if (open && !dialog?.open) dialog?.showModal();
  }, [open]);

  useEffect(() => {
    if (!done) return;
    const id = setTimeout(() => {
      setStep(step + 1);
      setDone(false);
    }, ADVANCE_MS);
    return () => clearTimeout(id);
  }, [done, step]);

  useEffect(() => {
    if (nudged === null) return;
    const id = setTimeout(() => setNudged(null), NUDGE_MS);
    return () => clearTimeout(id);
  }, [nudged]);

  const go = (to: number) => {
    stopDemo.current?.();
    setStep(to);
    setDone(false);
  };
  const onEvent = (e: BoardEvent) => {
    // What a demo does on the board is not the player's move.
    if (goal && !done && !stopDemo.current && reached(goal, e, boxes.current)) {
      setDone(true);
    }
    if ("boxes" in e) boxes.current = e.boxes;
  };
  // Pressed again while the demo plays, it stops it.
  const showMe = () => {
    if (stopDemo.current) return stopDemo.current();
    if (!canDemo(goal) || !board.current || !ghost.current) return;
    const script = demoScript(goal, board.current.boxes());
    setPlaying(true);
    setNudged(null);
    stopDemo.current = playDemo(board.current, ghost.current, script, (end) => {
      stopDemo.current = null;
      setPlaying(false);
      if (end) setNudged(step);
    });
  };
  // A real press, key or wheel on the board stops a demo before the board acts on it.
  // The player's pointer moves wait instead, so they never steer the demo's drag.
  const onInput = (e: SyntheticEvent) => {
    if (!stopDemo.current || !e.isTrusted) return;
    if (e.type === "pointermove") e.stopPropagation();
    else stopDemo.current();
  };
  const onReady = (b: Board | null) => {
    if (!b) stopDemo.current?.();
    board.current = b;
  };
  // Esc, the close button and the last step all end up here.
  const onClose = () => {
    markSeen();
    setReopened(false);
    go(0);
  };

  return (
    <>
      <button
        type="button"
        className="text-sm text-zinc-400 hover:text-white"
        onClick={() => setReopened(true)}
      >
        How to play
      </button>
      {/* Pinned near the top, so the board stays put as the step text changes length. */}
      <dialog
        ref={dialogRef}
        aria-labelledby="tutorial-title"
        onClose={onClose}
        className="mx-auto mt-[4vh] mb-auto w-[calc(100%-2rem)] max-w-lg rounded-xl border border-white/15 bg-black p-0 text-white backdrop:bg-black/70"
      >
        {open && (
          <div className="p-5">
            <div className="flex items-center justify-between">
              <p id="tutorial-title" className="text-sm text-zinc-400">
                How to play · {step + 1} of {STEPS.length}
              </p>
              <button
                type="button"
                aria-label="Close"
                className="-mr-2 rounded-md px-2 text-zinc-400 hover:text-white"
                onClick={() => dialogRef.current?.close()}
              >
                ✕
              </button>
            </div>
            <PracticeBoard
              onEvent={onEvent}
              onReady={onReady}
              onInput={onInput}
            >
              {/* The demo's pointer: a dot that shrinks and fills while pressed. */}
              <div
                ref={ghost}
                aria-hidden
                className="pointer-events-none absolute top-0 left-0 size-6 -translate-1/2 rounded-full border-2 border-white bg-white/20 opacity-0 shadow-[0_0_0_3px_rgba(0,0,0,0.5)] data-down:scale-75 data-down:bg-white/80 data-on:opacity-100 motion-safe:transition-[opacity,scale,background-color] motion-safe:duration-150"
              />
            </PracticeBoard>
            <div className="mt-4 flex items-center justify-between gap-3">
              <h2 className="font-semibold">
                {title}
                {done && <span className="text-zinc-400"> ✓</span>}
              </h2>
              {canDemo(goal) && (
                <button
                  type="button"
                  aria-pressed={playing}
                  disabled={done}
                  className="shrink-0 rounded-md border border-white/15 px-3 py-1 text-sm font-medium hover:bg-white/10 disabled:opacity-40 aria-pressed:border-white aria-pressed:bg-white aria-pressed:text-black aria-pressed:hover:bg-zinc-200"
                  onClick={showMe}
                >
                  Show me
                </button>
              )}
            </div>
            <div className="mt-1 text-base text-zinc-300">{body}</div>
            {turn && (
              <p
                className={`-mx-1.5 mt-2 rounded-md px-1.5 text-base motion-safe:transition-colors motion-safe:duration-500 ${nudged === step ? "bg-white/15 text-white" : "text-zinc-300"}`}
              >
                <strong className="font-semibold text-white">Your turn:</strong>{" "}
                {turn}
              </p>
            )}
            <div className="mt-5 flex items-center justify-between">
              <div className="flex gap-1.5" aria-hidden>
                {STEPS.map((_, i) => (
                  <span
                    key={i}
                    className={`size-1.5 rounded-full ${i === step ? "bg-white" : "bg-white/25"}`}
                  />
                ))}
              </div>
              <div className="flex gap-2">
                {step > 0 && (
                  <button
                    type="button"
                    className="rounded-md border border-white/15 px-4 py-2 text-sm font-medium hover:bg-white/10"
                    onClick={() => go(step - 1)}
                  >
                    Back
                  </button>
                )}
                <button
                  type="button"
                  className="rounded-md bg-white px-4 py-2 text-sm font-medium text-black hover:bg-zinc-200"
                  onClick={() =>
                    last ? dialogRef.current?.close() : go(step + 1)
                  }
                >
                  {last ? "Play" : "Next"}
                </button>
              </div>
            </div>
          </div>
        )}
      </dialog>
    </>
  );
}

/**
 * The practice puzzle on a board of its own: no timer, and solving it unlocks nothing.
 * `onReady` hears the board once it is up, and null just before it goes; `onInput` hears
 * presses, pointer moves, keys and wheels on it first; `children` go over it.
 */
function PracticeBoard({
  onEvent,
  onReady,
  onInput,
  children,
}: {
  onEvent: (e: BoardEvent) => void;
  onReady: (board: Board | null) => void;
  onInput: (e: SyntheticEvent) => void;
  children: ReactNode;
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const report = useEffectEvent(onEvent);
  const ready = useEffectEvent(onReady);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    let board: Board | null = null;
    let cancelled = false;
    mountBoard(canvas, PRACTICE, {
      onStatus: (s) => report({ boxes: s.boxes + s.wrong, solved: s.solved }),
      onView: (flat) => report({ flat }),
    })
      .then((b) => {
        if (cancelled) b.destroy();
        else {
          board = b;
          ready(b);
        }
      })
      .catch((e: unknown) => console.error(e));
    return () => {
      cancelled = true;
      if (board) ready(null);
      board?.destroy();
    };
  }, []);

  return (
    // font-mono: the board's clue labels take the font of this box.
    // At most 45vh, so the steps and buttons below it fit on a short screen too.
    // Capture handlers, so they run before the board's own listeners on the canvas.
    <div
      className="relative mx-auto mt-3 aspect-square w-full max-w-[45vh] overflow-hidden rounded-md bg-black font-mono"
      onPointerDownCapture={onInput}
      onPointerMoveCapture={onInput}
      onKeyDownCapture={onInput}
      onWheelCapture={onInput}
    >
      <canvas
        ref={canvasRef}
        className="h-full w-full cursor-grab touch-none outline-none focus-visible:ring-2 focus-visible:ring-white/40 focus-visible:ring-inset"
        tabIndex={0}
        aria-label="Practice board"
      />
      {children}
    </div>
  );
}

function Clues() {
  return (
    <>
      <p>
        Every box holds exactly one clue and takes its shape and volume. A
        marker has its box&apos;s shape; the number is the volume, and ? hides
        it.
      </p>
      <ul className="mt-3 grid grid-cols-2 gap-x-6 gap-y-1 sm:grid-cols-3">
        {(Object.keys(SHAPE_NAME) as Shape[]).map((s) => (
          <li key={s} className="flex items-center gap-2">
            <ShapeIcon shape={s} />
            {SHAPE_NAME[s]}
          </li>
        ))}
        <li className="flex items-center gap-2">
          <ShapeIcon />
          any shape
        </li>
      </ul>
    </>
  );
}

function Tips() {
  return (
    <ul className="list-disc space-y-1 pl-5">
      <li>
        Pause on a cell mid-drag to keep it in the box, then drag on to grow the
        box another way.
      </li>
      <li>
        Or click one cell, then another. In the flat view, change layer between
        the clicks to build through several layers.
      </li>
      <li>
        Keyboard: Tab to the board. Arrows move, Shift+↑/↓ changes layer, Space
        starts and places a box, Delete removes, Esc cancels, Shift+←/→ turns,
        +/− zoom.
      </li>
    </ul>
  );
}

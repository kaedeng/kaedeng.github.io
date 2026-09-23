import init, { Game } from "../wasm/patches_wasm.js";
import { clueColor, clueText, PALETTE, type Puzzle } from "./puzzle.js";

export * from "./puzzle.js";

/** Boxes that keep every rule, boxes that break one, and whether the puzzle is solved. */
export type Status = { boxes: number; wrong: number; solved: boolean };

export type BoardOptions = {
  /** Show the stored solution; the cube still turns and zooms but takes no boxes. */
  answer?: boolean;
  /** Called once the board is up, and after every change to it. */
  onStatus?: (status: Status) => void;
  /** Called on every press on a cell and every board key, e.g. to start a timer. */
  onPlay?: () => void;
};

export type Board = {
  /** Clears every placed box. */
  reset(): void;
  /** Stops listening, removes the overlay and frees the board. */
  destroy(): void;
};

type Overlay = {
  root: HTMLDivElement;
  labels: HTMLSpanElement[];
  modeLine: HTMLParagraphElement;
};

type Listeners = {
  [K in keyof HTMLElementEventMap]?: (e: HTMLElementEventMap[K]) => void;
};

// The .wasm is fetched from next to the wasm-bindgen glue (`import.meta.url`), so it
// works under any base path and with any bundler that understands `new URL`.
let wasmReady: Promise<unknown> | null = null;

/**
 * Draws `puzzle` on `canvas` and forwards the canvas's pointer, wheel, key and focus
 * events to it. Clue labels and the vim mode line go in an overlay added right after the
 * canvas: the canvas should fill a positioned parent, and the labels take its font. The
 * drawing buffer is sized to the canvas once, here.
 */
export async function mountBoard(
  canvas: HTMLCanvasElement,
  puzzle: Puzzle,
  options: BoardOptions = {},
): Promise<Board> {
  wasmReady ??= init();
  await wasmReady;
  const dpr = window.devicePixelRatio || 1;
  canvas.width = Math.round(canvas.clientWidth * dpr);
  canvas.height = Math.round(canvas.clientHeight * dpr);
  const palette = new Uint32Array(PALETTE.map((c) => parseInt(c.slice(1), 16)));
  const game = new Game(
    canvas,
    JSON.stringify(puzzle),
    options.answer ?? false,
    palette,
  );
  const overlay = createOverlay(puzzle);
  canvas.after(overlay.root);

  let raf = 0;
  let last = 0;
  // Draws on animation frames until the tweens settle; no frames while idle.
  const animate = () => {
    if (raf) return;
    const frame = (t: number) => {
      const dt = last ? t - last : 0;
      last = t;
      const more = game.tick(dt);
      game.render();
      raf = more ? requestAnimationFrame(frame) : 0;
      if (!more) last = 0;
    };
    raf = requestAnimationFrame(frame);
  };
  const refresh = (boardChanged: boolean) => {
    placeLabels(game, overlay.labels);
    if (boardChanged) {
      options.onStatus?.({
        boxes: game.box_count(),
        wrong: game.wrong_count(),
        solved: game.is_solved(),
      });
    }
    animate();
  };

  const listeners = Object.entries(
    inputListeners(canvas, game, overlay, refresh, options),
  ) as [string, EventListener][];
  for (const [type, fn] of listeners) {
    // Not passive, so a zooming wheel can stop the page from scrolling.
    canvas.addEventListener(type, fn, { passive: false });
  }
  refresh(true);

  return {
    reset() {
      game.reset();
      refresh(true);
    },
    destroy() {
      for (const [type, fn] of listeners) canvas.removeEventListener(type, fn);
      cancelAnimationFrame(raf);
      overlay.root.remove();
      canvas.style.cursor = "";
      game.free();
    },
  };
}

function inputListeners(
  canvas: HTMLCanvasElement,
  game: Game,
  overlay: Overlay,
  refresh: (boardChanged: boolean) => void,
  options: BoardOptions,
): Listeners {
  const point = (e: PointerEvent) => [e.offsetX, e.offsetY] as const;
  const showHover = () => {
    canvas.style.cursor = game.hovering() ? "pointer" : "";
  };
  return {
    // Wheel and trackpad pinch (ctrl+wheel) zoom; at either end the page scrolls instead.
    wheel: (e) => {
      const speed = e.ctrlKey ? 0.01 : 0.0015;
      if (!game.zoom_by(-e.deltaY * speed)) return;
      e.preventDefault();
      refresh(false);
    },
    keydown: (e) => {
      if (e.metaKey || e.ctrlKey || e.altKey) return;
      if (!game.key(e.key, e.shiftKey)) return;
      e.preventDefault();
      options.onPlay?.();
      overlay.modeLine.textContent = game.mode_line();
      refresh(true);
    },
    focus: () => {
      game.set_cursor_visible(canvas.matches(":focus-visible"));
      refresh(false);
    },
    blur: () => {
      game.set_cursor_visible(false);
      refresh(false);
    },
    pointerdown: (e) => {
      canvas.setPointerCapture(e.pointerId);
      if (game.pointer_down(...point(e))) options.onPlay?.();
      refresh(false);
    },
    pointermove: (e) => {
      if (game.pointer_move(...point(e), e.timeStamp)) refresh(false);
      showHover();
    },
    pointerup: (e) => {
      refresh(game.pointer_up(...point(e)));
      showHover();
    },
    pointerleave: () => {
      if (game.pointer_leave()) refresh(false);
    },
    pointercancel: () => {
      game.pointer_cancel();
      refresh(false);
    },
  };
}

function createOverlay(puzzle: Puzzle): Overlay {
  const root = document.createElement("div");
  root.style.cssText = "position:absolute;inset:0;pointer-events:none";
  const labels = puzzle.clues.map((clue, i) => {
    const span = document.createElement("span");
    span.textContent = clueText(clue);
    span.style.cssText =
      "position:absolute;top:0;left:0;display:none;padding:0 0.25rem;" +
      "border-radius:0.25rem;font-size:0.875rem;line-height:1.25rem;" +
      `font-weight:600;color:#000;background:${clueColor(i)}`;
    return span;
  });
  const modeLine = document.createElement("p");
  modeLine.style.cssText =
    "position:absolute;bottom:0.75rem;left:1rem;margin:0;" +
    "font-size:0.875rem;line-height:1.25rem;color:#d4d4d8";
  root.append(...labels, modeLine);
  return { root, labels, modeLine };
}

function placeLabels(game: Game, spans: HTMLSpanElement[]) {
  const points = game.labels();
  spans.forEach((span, i) => {
    const [x, y] = [points[2 * i], points[2 * i + 1]];
    // NaN: the clue is in a peeled layer.
    span.style.display = Number.isNaN(x) ? "none" : "";
    span.style.transform = `translate(-50%, -50%) translate(${x}px, ${y}px)`;
  });
}

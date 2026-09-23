import init, { Game, generate } from "../wasm/patches_wasm.js";
import { clueColors, clueText, type Puzzle } from "./puzzle.js";

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
  /** Called when the camera turns, or switches between 3D and a flat 2D layer. */
  onView?: (flat: boolean) => void;
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
  /** Turns with the camera; clicking a face shows one layer face-on. */
  viewCube: HTMLDivElement;
  faces: HTMLButtonElement[];
  /** Pages through the layers in 2D; hidden in 3D. */
  layerBar: HTMLDivElement;
  layerText: HTMLSpanElement;
  nearer: HTMLButtonElement;
  deeper: HTMLButtonElement;
};

/** The view cube's faces: name, the axis they're across (x, y, z), which side, placement. */
const FACES = [
  ["Front", 2, 1, "translateZ(24px)"],
  ["Right", 0, 1, "rotateY(90deg) translateZ(24px)"],
  ["Top", 1, 1, "rotateX(90deg) translateZ(24px)"],
  ["Back", 2, -1, "rotateY(180deg) translateZ(24px)"],
  ["Left", 0, -1, "rotateY(-90deg) translateZ(24px)"],
  ["Bottom", 1, -1, "rotateX(-90deg) translateZ(24px)"],
] as const;

type Listeners = {
  [K in keyof HTMLElementEventMap]?: (e: HTMLElementEventMap[K]) => void;
};

// The .wasm is fetched from next to the wasm-bindgen glue (`import.meta.url`), so it
// works under any base path and with any bundler that understands `new URL`.
let wasmReady: Promise<unknown> | null = null;

/** A uniquely solvable puzzle for `seed`: the same one the `generate` CLI prints. */
export async function generatePuzzle(seed: string): Promise<Puzzle> {
  wasmReady ??= init();
  await wasmReady;
  return JSON.parse(generate(seed)) as Puzzle;
}

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
  const colors = clueColors(puzzle.clues);
  const game = new Game(
    canvas,
    JSON.stringify(puzzle),
    options.answer ?? false,
    new Uint32Array(colors.map((c) => parseInt(c.slice(1), 16))),
  );
  const overlay = createOverlay(puzzle, colors);
  canvas.after(overlay.root);

  let raf = 0;
  let last = 0;
  let view = "";
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
    showView(game, overlay, puzzle.size);
    const now = `${game.view_angles()} ${game.view_depth()}`;
    if (view && now !== view) options.onView?.(game.view_depth() >= 0);
    view = now;
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
  overlay.faces.forEach((face, i) => {
    face.onclick = () => {
      game.view_face(FACES[i][1], FACES[i][2]);
      // So the arrow keys go on to move the cursor.
      canvas.focus({ preventScroll: true });
      refresh(false);
    };
  });
  overlay.nearer.onclick = () => {
    game.step_layer(-1);
    refresh(false);
  };
  overlay.deeper.onclick = () => {
    game.step_layer(1);
    refresh(false);
  };
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

function createOverlay(puzzle: Puzzle, colors: string[]): Overlay {
  const root = document.createElement("div");
  root.style.cssText = "position:absolute;inset:0;pointer-events:none";
  const labels = puzzle.clues.map((clue, i) => {
    const span = document.createElement("span");
    span.textContent = clueText(clue);
    span.style.cssText =
      "position:absolute;top:0;left:0;display:none;padding:0 0.25rem;" +
      "border-radius:0.25rem;font-size:0.875rem;line-height:1.25rem;" +
      `font-weight:600;color:#000;background:${colors[i]}`;
    return span;
  });
  const modeLine = document.createElement("p");
  modeLine.style.cssText =
    "position:absolute;bottom:0.75rem;left:1rem;margin:0;" +
    "font-size:0.875rem;line-height:1.25rem;color:#d4d4d8";
  const { box, viewCube, faces } = createViewCube();
  const layerBar = document.createElement("div");
  layerBar.style.cssText =
    "position:absolute;bottom:0.75rem;right:0.75rem;display:none;" +
    "align-items:center;gap:0.25rem;pointer-events:auto;" +
    "font-size:0.875rem;line-height:1.25rem;color:#d4d4d8";
  const nearer = pagerButton("‹", "Nearer layer");
  const deeper = pagerButton("›", "Deeper layer");
  const layerText = document.createElement("span");
  layerText.style.cssText =
    "padding:0 0.25rem;font-variant-numeric:tabular-nums";
  layerBar.append(nearer, layerText, deeper);
  root.append(...labels, modeLine, box, layerBar);
  return {
    root,
    labels,
    modeLine,
    viewCube,
    faces,
    layerBar,
    layerText,
    nearer,
    deeper,
  };
}

/**
 * A 48 px cube in the top-right corner, drawn with CSS 3D transforms. Turned, it reaches
 * up to 18 px past its box, hence the inset.
 */
function createViewCube() {
  const box = document.createElement("div");
  box.style.cssText =
    "position:absolute;top:1.75rem;right:1.75rem;width:48px;height:48px";
  // Only the faces take the pointer: the turned container's own plane cuts through the
  // cube and would catch clicks meant for the far half of a face.
  const viewCube = document.createElement("div");
  viewCube.style.cssText =
    "position:absolute;inset:0;transform-style:preserve-3d";
  const faces = FACES.map(([name, , , place]) => {
    const face = document.createElement("button");
    face.type = "button";
    face.textContent = name;
    face.style.cssText =
      "position:absolute;inset:0;display:grid;place-items:center;padding:0;" +
      "font:inherit;font-size:9px;font-weight:600;letter-spacing:0.04em;" +
      "text-transform:uppercase;color:#a1a1aa;background:rgba(10,10,10,0.85);" +
      "border:1px solid rgba(255,255,255,0.25);cursor:pointer;pointer-events:auto;" +
      `backface-visibility:hidden;transform:${place}`;
    face.onpointerenter = () => (face.style.color = "#fff");
    face.onpointerleave = () => (face.style.color = "#a1a1aa");
    return face;
  });
  viewCube.append(...faces);
  box.append(viewCube);
  return { box, viewCube, faces };
}

function pagerButton(text: string, label: string): HTMLButtonElement {
  const b = document.createElement("button");
  b.type = "button";
  b.textContent = text;
  b.ariaLabel = label;
  b.style.cssText =
    "width:1.75rem;height:1.75rem;padding:0;font:inherit;color:inherit;" +
    "background:none;border:1px solid rgba(255,255,255,0.15);border-radius:0.375rem;" +
    "cursor:pointer";
  return b;
}

/** Turns the view cube with the camera, and shows the layer bar in 2D. */
function showView(game: Game, overlay: Overlay, size: number) {
  const [yaw, pitch] = game.view_angles();
  overlay.viewCube.style.transform = `rotateX(${-pitch}rad) rotateY(${-yaw}rad)`;
  const depth = game.view_depth();
  const flat = depth >= 0;
  overlay.faces.forEach((face, i) => {
    face.title = flat ? "Back to 3D" : `${FACES[i][0]} layer in 2D`;
  });
  overlay.layerBar.style.display = flat ? "flex" : "none";
  overlay.layerText.textContent = `Layer ${depth + 1} of ${size}`;
  overlay.nearer.disabled = depth <= 0;
  overlay.deeper.disabled = depth >= size - 1;
  for (const b of [overlay.nearer, overlay.deeper]) {
    b.style.opacity = b.disabled ? "0.35" : "";
  }
}

function placeLabels(game: Game, spans: HTMLSpanElement[]) {
  const points = game.labels();
  const hidden = game.hidden_labels();
  spans.forEach((span, i) => {
    const [x, y] = [points[2 * i], points[2 * i + 1]];
    // NaN: the clue is in a peeled layer.
    span.style.display = Number.isNaN(x) ? "none" : "";
    // Behind another box: still there, but it no longer reads as on the front.
    span.style.opacity = hidden[i] ? "0.35" : "";
    span.style.transform = `translate(-50%, -50%) translate(${x}px, ${y}px)`;
  });
}

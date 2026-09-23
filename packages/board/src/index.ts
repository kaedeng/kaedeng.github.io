import init, { daily, Game, generate } from "../wasm/patches_wasm.js";
import { Fingers } from "./fingers.js";
import {
  labelFont,
  viewCubeInset,
  viewCubePerspective,
  viewCubeSide,
} from "./sizes.js";
import {
  clueColors,
  clueText,
  type Box,
  type Cell,
  type Puzzle,
} from "./puzzle.js";

export * from "./puzzle.js";

/**
 * Boxes that keep every rule, boxes that break one, how many boxes are locked, and whether
 * the puzzle is solved.
 */
export type Status = {
  boxes: number;
  wrong: number;
  locked: number;
  solved: boolean;
};

/** A box on the board, and whether the player locked it. */
export type PlacedBox = Box & { locked: boolean };

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
  /**
   * Feeds a pointer event to the same handlers as the canvas's own listeners, e.g. for a
   * demo: `x`, `y` in CSS px from the canvas's top-left, `time` in ms like `timeStamp`.
   */
  pointer(type: PointerType, x: number, y: number, time: number): void;
  /** Where `cell`'s centre is on the canvas, in CSS px; null while its layer is hidden. */
  cellPoint(cell: Cell): [number, number] | null;
  /** The view-cube face turned most toward the viewer, e.g. for a demo to click. */
  nearestFace(): HTMLButtonElement;
  /** The placed boxes. */
  boxes(): PlacedBox[];
  /** Puts the placed boxes back to `boxes`, e.g. after a demo, and calls `onStatus`. */
  setBoxes(boxes: PlacedBox[]): void;
};

/** The pointer events `Board.pointer` takes, after `pointerdown` and so on. */
export type PointerType = "down" | "move" | "up" | "leave" | "cancel";

/** What the canvas's pointer listeners do with a pointer at `x`, `y` at `time`. */
type Pointer = Record<
  PointerType,
  (x: number, y: number, time: number) => void
>;

/** How long (ms) a press held still on a box takes to lock or unlock it. */
const HOLD_MS = 500;

/** A padlock, drawn in the colour of the element it is in. */
const LOCK_SVG =
  '<svg viewBox="0 0 16 16" width="100%" height="100%" aria-hidden="true">' +
  '<path d="M5 7.5V5a3 3 0 0 1 6 0v2.5" fill="none" stroke="currentColor" stroke-width="2"/>' +
  '<rect x="2.5" y="7" width="11" height="8" rx="1.5" fill="currentColor"/></svg>';

type Overlay = {
  root: HTMLDivElement;
  labels: HTMLSpanElement[];
  /** One padlock per locked box in view, made as needed; the spare ones are hidden. */
  locks: HTMLSpanElement[];
  modeLine: HTMLParagraphElement;
  /** Turns with the camera; clicking a face shows one layer face-on. */
  viewCube: HTMLDivElement;
  /** The view cube's box in the corner, which sets its size and perspective. */
  viewBox: HTMLDivElement;
  /** The view cube's side, CSS px. */
  cubeSide: number;
  faces: HTMLButtonElement[];
  /** Pages through the layers in 2D; hidden in 3D. */
  layerBar: HTMLDivElement;
  layerText: HTMLSpanElement;
  nearer: HTMLButtonElement;
  deeper: HTMLButtonElement;
};

/** The view cube's faces: name, the axis they're across (x, y, z), which side, turn. */
const FACES = [
  ["Front", 2, 1, ""],
  ["Right", 0, 1, "rotateY(90deg)"],
  ["Top", 1, 1, "rotateX(90deg)"],
  ["Back", 2, -1, "rotateY(180deg)"],
  ["Left", 0, -1, "rotateY(-90deg)"],
  ["Bottom", 1, -1, "rotateX(-90deg)"],
] as const;

type Listeners = {
  [K in keyof HTMLElementEventMap]?: (e: HTMLElementEventMap[K]) => void;
};

// The .wasm is fetched from next to the wasm-bindgen glue (`import.meta.url`), so it
// works under any base path and with any bundler that understands `new URL`.
let wasmReady: Promise<unknown> | null = null;

/**
 * A uniquely solvable puzzle for `id`, a seed plus `-easy` or `-hard` for those levels:
 * the same one the `generate` CLI prints. Its `id` is the canonical spelling.
 */
export async function generatePuzzle(id: string): Promise<Puzzle> {
  wasmReady ??= init();
  await wasmReady;
  return JSON.parse(generate(id)) as Puzzle;
}

/**
 * The daily puzzle for `date` (`2026-09-23`): the one `generate --daily <date>` prints.
 * No id `generatePuzzle` takes makes it, not even the same date.
 */
export async function dailyPuzzle(date: string): Promise<Puzzle> {
  wasmReady ??= init();
  await wasmReady;
  return JSON.parse(daily(date)) as Puzzle;
}

/**
 * Draws `puzzle` on `canvas` and forwards the canvas's pointer, wheel, key and focus
 * events to it. Clue labels and the vim mode line go in an overlay added right after the
 * canvas: the canvas should fill a positioned parent, and the labels take its font. The
 * drawing buffer follows the canvas's size, so the canvas may resize freely.
 */
export async function mountBoard(
  canvas: HTMLCanvasElement,
  puzzle: Puzzle,
  options: BoardOptions = {},
): Promise<Board> {
  wasmReady ??= init();
  await wasmReady;
  fitBuffer(canvas);
  const colors = clueColors(puzzle.clues);
  const game = new Game(
    canvas,
    JSON.stringify(puzzle),
    options.answer ?? false,
    new Uint32Array(colors.map((c) => parseInt(c.slice(1), 16))),
  );
  const overlay = createOverlay(puzzle, colors);
  sizeOverlay(overlay, canvas.clientWidth);
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
    placeLocks(game, overlay, canvas.clientWidth);
    showView(game, overlay, puzzle.size);
    const now = `${game.view_angles()} ${game.view_depth()}`;
    if (view && now !== view) options.onView?.(game.view_depth() >= 0);
    view = now;
    if (boardChanged) {
      options.onStatus?.({
        boxes: game.box_count(),
        wrong: game.wrong_count(),
        locked: game.locked_count(),
        solved: game.is_solved(),
      });
    }
    animate();
  };

  const pointer = pointerHandlers(canvas, game, refresh, options);
  const listeners = Object.entries(
    inputListeners(canvas, game, overlay, refresh, options, pointer),
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
  // When the window resizes or a phone turns, a buffer left at the old size would stretch
  // the cube and pull the labels off their cells.
  const resized = new ResizeObserver(() => {
    if (!fitBuffer(canvas)) return;
    game.resize();
    sizeOverlay(overlay, canvas.clientWidth);
    refresh(false);
  });
  resized.observe(canvas);
  refresh(true);

  return {
    reset() {
      game.reset();
      refresh(true);
    },
    destroy() {
      pointer.stop();
      resized.disconnect();
      for (const [type, fn] of listeners) canvas.removeEventListener(type, fn);
      cancelAnimationFrame(raf);
      overlay.root.remove();
      canvas.style.cursor = "";
      game.free();
    },
    pointer(type, x, y, time) {
      pointer[type](x, y, time);
    },
    cellPoint(cell) {
      const [x, y] = game.cell_point(...cell);
      return Number.isNaN(x) ? null : [x, y];
    },
    nearestFace() {
      const [axis, sign] = game.facing();
      return overlay.faces[
        FACES.findIndex(([, a, s]) => a === axis && s === sign)
      ];
    },
    boxes() {
      const flat = [...game.boxes()];
      return Array.from({ length: flat.length / 7 }, (_, i) => {
        const [x0, y0, z0, x1, y1, z1, locked] = flat.slice(7 * i, 7 * i + 7);
        return { min: [x0, y0, z0], max: [x1, y1, z1], locked: locked === 1 };
      });
    },
    setBoxes(boxes) {
      game.set_boxes(
        new Uint8Array(
          boxes.flatMap((b) => [...b.min, ...b.max, Number(b.locked)]),
        ),
      );
      refresh(true);
    },
  };
}

/** The pointer handlers, and `stop`, which drops a hold still timing, e.g. on destroy. */
function pointerHandlers(
  canvas: HTMLCanvasElement,
  game: Game,
  refresh: (boardChanged: boolean) => void,
  options: BoardOptions,
): Pointer & { stop: () => void } {
  const showHover = () => {
    canvas.style.cursor = game.hovering() ? "pointer" : "";
  };
  // A press held still on a box locks or unlocks it once this goes off.
  let hold = 0;
  const stop = () => clearTimeout(hold);
  return {
    down: (x, y) => {
      if (game.pointer_down(x, y)) options.onPlay?.();
      stop();
      hold = window.setTimeout(() => {
        if (game.pointer_hold()) refresh(true);
      }, HOLD_MS);
      refresh(false);
    },
    move: (x, y, time) => {
      if (game.pointer_move(x, y, time)) refresh(false);
      showHover();
    },
    up: (x, y) => {
      stop();
      refresh(game.pointer_up(x, y));
      showHover();
    },
    leave: () => {
      if (game.pointer_leave()) refresh(false);
    },
    cancel: () => {
      stop();
      game.pointer_cancel();
      refresh(false);
    },
    stop,
  };
}

function inputListeners(
  canvas: HTMLCanvasElement,
  game: Game,
  overlay: Overlay,
  refresh: (boardChanged: boolean) => void,
  options: BoardOptions,
  pointer: Pointer,
): Listeners {
  const point = (e: PointerEvent) =>
    [e.offsetX, e.offsetY, e.timeStamp] as const;
  // Two fingers pinch to zoom; one finger or a mouse presses as before.
  const fingers = new Fingers();
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
    // The board's own right-click and long press take the place of the page's menu.
    contextmenu: (e) => e.preventDefault(),
    pointerdown: (e) => {
      // A right-click locks or unlocks the box under it, and is no press.
      if (e.button === 2) {
        if (!game.toggle_lock(e.offsetX, e.offsetY)) return;
        options.onPlay?.();
        refresh(true);
        return;
      }
      canvas.setPointerCapture(e.pointerId);
      const what = fingers.down(e.pointerId, e.offsetX, e.offsetY);
      if (what === "press") pointer.down(...point(e));
      if (what === "pinch") pointer.cancel(...point(e));
    },
    pointermove: (e) => {
      const zoom = fingers.move(e.pointerId, e.offsetX, e.offsetY);
      if (zoom === null) pointer.move(...point(e));
      else if (game.zoom_by(zoom)) refresh(false);
    },
    pointerup: (e) => {
      if (fingers.up(e.pointerId)) pointer.up(...point(e));
    },
    pointerleave: (e) => pointer.leave(...point(e)),
    pointercancel: (e) => {
      fingers.up(e.pointerId);
      pointer.cancel(...point(e));
    },
  };
}

/**
 * Sizes the drawing buffer to the canvas's CSS size times `devicePixelRatio`. Returns
 * false when it already fits, or while the canvas is hidden and has no size.
 */
function fitBuffer(canvas: HTMLCanvasElement): boolean {
  const dpr = window.devicePixelRatio || 1;
  const w = Math.round(canvas.clientWidth * dpr);
  const h = Math.round(canvas.clientHeight * dpr);
  if (!w || !h || (w === canvas.width && h === canvas.height)) return false;
  canvas.width = w;
  canvas.height = h;
  return true;
}

/** Labels and the view cube are smaller on a small board; `width` is the canvas's. */
function sizeOverlay(overlay: Overlay, width: number) {
  const font = labelFont(width);
  for (const span of overlay.labels) {
    span.style.padding = `0 ${font * 0.3}px`;
    span.style.borderRadius = `${font * 0.3}px`;
    span.style.fontSize = `${font}px`;
    span.style.lineHeight = `${font * 1.4}px`;
  }
  const side = viewCubeSide(width);
  overlay.cubeSide = side;
  Object.assign(overlay.viewBox.style, {
    width: `${side}px`,
    height: `${side}px`,
    top: `${viewCubeInset(side)}px`,
    right: `${viewCubeInset(side)}px`,
  });
  overlay.faces.forEach((face, i) => {
    face.style.transform = `${FACES[i][3]} translateZ(${side / 2}px)`;
    face.style.fontSize = `${Math.max(7, side * 0.19)}px`;
  });
}

function createOverlay(puzzle: Puzzle, colors: string[]): Overlay {
  const root = document.createElement("div");
  root.style.cssText = "position:absolute;inset:0;pointer-events:none";
  const labels = puzzle.clues.map((clue, i) => {
    const span = document.createElement("span");
    span.textContent = clueText(clue);
    span.style.cssText =
      "position:absolute;top:0;left:0;display:none;" +
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
    locks: [],
    modeLine,
    viewCube,
    viewBox: box,
    cubeSide: 0,
    faces,
    layerBar,
    layerText,
    nearer,
    deeper,
  };
}

/**
 * A cube in the top-right corner, drawn with CSS 3D transforms; `sizeOverlay` sizes it
 * and places its faces.
 */
function createViewCube() {
  const box = document.createElement("div");
  box.style.position = "absolute";
  // Only the faces take the pointer: the turned container's own plane cuts through the
  // cube and would catch clicks meant for the far half of a face.
  const viewCube = document.createElement("div");
  viewCube.style.cssText =
    "position:absolute;inset:0;transform-style:preserve-3d";
  const faces = FACES.map(([name]) => {
    const face = document.createElement("button");
    face.type = "button";
    face.textContent = name;
    face.style.cssText =
      "position:absolute;inset:0;display:grid;place-items:center;padding:0;" +
      "font:inherit;font-weight:600;letter-spacing:0.04em;" +
      "text-transform:uppercase;color:#a1a1aa;background:rgba(10,10,10,0.85);" +
      "border:1px solid rgba(255,255,255,0.25);cursor:pointer;pointer-events:auto;" +
      "backface-visibility:hidden";
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
  // Seen as the board is: with the camera's perspective in 3D, straight on in 2D.
  overlay.viewBox.style.perspective = flat
    ? "none"
    : `${viewCubePerspective(overlay.cubeSide, game.view_distance(), size)}px`;
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

/**
 * A padlock on each locked box in view, sized like the labels on a board `width` px wide;
 * none where another box covers any of it.
 */
function placeLocks(game: Game, overlay: Overlay, width: number) {
  const size = labelFont(width) * 1.6;
  const locks = game.locks(size / 2);
  const count = locks.length / 3;
  while (overlay.locks.length < count) {
    const span = document.createElement("span");
    span.style.cssText = "position:absolute;top:0;left:0";
    span.innerHTML = LOCK_SVG;
    overlay.locks.push(span);
    // Under the labels, so a clue's number stays readable.
    overlay.root.prepend(span);
  }
  overlay.locks.forEach((span, i) => {
    span.style.display = i < count ? "" : "none";
    if (i >= count) return;
    const [x, y, rgb] = locks.slice(3 * i, 3 * i + 3);
    span.style.width = span.style.height = `${size}px`;
    span.style.color = `#${rgb.toString(16).padStart(6, "0")}`;
    span.style.transform = `translate(-50%, -50%) translate(${x}px, ${y}px)`;
  });
}

function placeLabels(game: Game, spans: HTMLSpanElement[]) {
  const points = game.labels();
  const hidden = game.hidden_labels();
  spans.forEach((span, i) => {
    const [x, y] = [points[2 * i], points[2 * i + 1]];
    // NaN: the clue is in a peeled layer. Behind another box, it would float over that
    // box's face.
    span.style.display = Number.isNaN(x) || hidden[i] ? "none" : "";
    span.style.transform = `translate(-50%, -50%) translate(${x}px, ${y}px)`;
  });
}

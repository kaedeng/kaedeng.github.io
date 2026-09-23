import type { Board } from "patches-board";
import { HOME, MAX_FRAME, pointAt, type Action, type Aim } from "./demo";

type Point = [number, number];

/**
 * Plays `script` on `board`, with `ghost` (an element over the canvas, in the same box)
 * as the pointer. The board takes the ghost's presses and moves through its own pointer
 * handlers, and the ghost clicks the real view-cube face, so a demo does just what the
 * player's input would. When the script ends, or the returned function stops it, the
 * board is put back: the ghost lets go, a 2D view it opened closes, and the boxes are as
 * they were. `onEnd` then hears whether the script ran to the end.
 */
export function playDemo(
  board: Board,
  ghost: HTMLElement,
  script: Action[],
  onEnd: (finished: boolean) => void,
): () => void {
  const area = ghost.parentElement as HTMLElement;
  const before = board.boxes();
  let at: Point = [HOME[0] * area.clientWidth, HOME[1] * area.clientHeight];
  let down = false;
  let onFace = false;
  // Face clicks: after an odd number, a 2D view the demo opened is still showing.
  let clicks = 0;
  // Time for the board's pointer moves; see MAX_FRAME.
  let clock = performance.now();
  let raf = 0;
  let over = false;

  const show = () => {
    ghost.style.left = `${at[0]}px`;
    ghost.style.top = `${at[1]}px`;
    ghost.toggleAttribute("data-down", down);
  };

  const resolve = (aim: Aim): Point => {
    if (aim === "face") return center(board.nearestFace(), area);
    if ("cell" in aim) return board.cellPoint(aim.cell) ?? at;
    if ("by" in aim) return [at[0] + aim.by[0], at[1] + aim.by[1]];
    return [aim.at[0] * area.clientWidth, aim.at[1] * area.clientHeight];
  };

  // Calls `each` with 0 to 1 on animation frames, over `ms` of the demo's clock.
  const frames = (ms: number, each: (t: number) => void) =>
    new Promise<void>((done) => {
      let t = 0;
      let last = performance.now();
      const frame = (now: number) => {
        const dt = Math.min(Math.max(now - last, 0), MAX_FRAME);
        last = now;
        t += dt;
        clock += dt;
        each(Math.min(t / ms, 1));
        if (t < ms) raf = requestAnimationFrame(frame);
        else done();
      };
      raf = requestAnimationFrame(frame);
    });

  const glide = (aim: Aim, ms: number) => {
    onFace = aim === "face";
    const from = at;
    const to = resolve(aim);
    return frames(ms, (t) => {
      at = pointAt(from, to, t);
      show();
      // The view cube sits over the canvas, so the canvas never sees the pointer there.
      if (!onFace) board.pointer("move", ...at, clock);
    });
  };

  const press = (pressed: boolean) => {
    down = pressed;
    show();
    if (!onFace) board.pointer(pressed ? "down" : "up", ...at, clock);
    else if (!pressed) {
      clicks++;
      board.nearestFace().click();
    }
  };

  const step = (a: Action) => {
    if ("wait" in a) return frames(a.wait, () => {});
    if ("down" in a) return press(a.down);
    return glide(a.to, a.ms);
  };

  const end = (finished: boolean) => {
    if (over) return;
    over = true;
    cancelAnimationFrame(raf);
    // Lets go mid-drag, and leaves the board, so no hover stays behind.
    if (down) board.pointer("cancel", ...at, clock);
    board.pointer("leave", ...at, clock);
    if (clicks % 2) board.nearestFace().click();
    board.setBoxes(before);
    ghost.removeAttribute("data-on");
    ghost.removeAttribute("data-down");
    onEnd(finished);
  };

  const run = async () => {
    show();
    ghost.setAttribute("data-on", "");
    for (const a of script) {
      await step(a);
      // Stopped: `end` has already put the board back.
      if (over) return;
    }
    end(true);
  };

  void run();
  return () => end(false);
}

/** The centre of `el` in `area`'s CSS px. */
function center(el: Element, area: Element): Point {
  const r = el.getBoundingClientRect();
  const a = area.getBoundingClientRect();
  return [r.x + r.width / 2 - a.x, r.y + r.height / 2 - a.y];
}

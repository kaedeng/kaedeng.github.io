/** Zoom (the full range is 1) for fingers spreading to `e` times as far apart. */
const PINCH_SPEED = 1.5;

/**
 * The pointers down on the board, so two fingers pinch to zoom. One finger (or a mouse)
 * passes straight through as a press; a second one turns it into a pinch. Once pinching,
 * the fingers do nothing else until every one has lifted, so a finger left behind can't
 * build or remove a box.
 */
export class Fingers {
  #at = new Map<number, [number, number]>();
  #spread = 0;
  #pinching = false;

  /** What a new pointer does: start a press, start a pinch (cancel the press), or nothing. */
  down(id: number, x: number, y: number): "press" | "pinch" | "ignore" {
    if (this.#at.size >= 2) return "ignore";
    this.#at.set(id, [x, y]);
    if (this.#at.size === 1 && !this.#pinching) return "press";
    this.#pinching = true;
    this.#spread = this.#measure();
    return "pinch";
  }

  /** While pinching, the zoom change (possibly 0); else null, for the press to move. */
  move(id: number, x: number, y: number): number | null {
    if (!this.#pinching) return null;
    if (!this.#at.has(id) || this.#at.size < 2) return 0;
    this.#at.set(id, [x, y]);
    const spread = this.#measure();
    const zoom = Math.log(spread / this.#spread) * PINCH_SPEED;
    this.#spread = spread;
    return Number.isFinite(zoom) ? zoom : 0;
  }

  /** Whether the press should hear this pointer lift: not after a pinch. */
  up(id: number): boolean {
    this.#at.delete(id);
    const press = !this.#pinching;
    if (this.#at.size === 0) this.#pinching = false;
    return press;
  }

  /** Distance between the first two fingers, CSS px. */
  #measure(): number {
    const [a, b] = [...this.#at.values()];
    return Math.hypot(a[0] - b[0], a[1] - b[1]);
  }
}

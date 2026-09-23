/** What a practice step asks the player to do on the board. */
export type Goal = "place" | "remove" | "lock" | "turn" | "flat" | "solve";

/** What the practice board reports: its boxes after a change, or the camera after a move. */
export type BoardEvent =
  { boxes: number; locked: number; solved: boolean } | { flat: boolean };

/**
 * Whether `event` completes `goal`; `boxes` is how many boxes were placed before it, and
 * `locked` how many of them were locked.
 */
export function reached(
  goal: Goal,
  event: BoardEvent,
  boxes: number,
  locked: number,
): boolean {
  if ("flat" in event) {
    return goal === "flat" ? event.flat : goal === "turn" && !event.flat;
  }
  switch (goal) {
    case "place":
      return event.boxes > boxes;
    case "remove":
      return event.boxes < boxes;
    case "lock":
      return event.locked > locked;
    case "solve":
      return event.solved;
    default:
      return false;
  }
}

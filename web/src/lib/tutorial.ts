/** What a practice step asks the player to do on the board. */
export type Goal = "place" | "remove" | "turn" | "flat" | "solve";

/** What the practice board reports: its boxes after a change, or the camera after a move. */
export type BoardEvent = { boxes: number; solved: boolean } | { flat: boolean };

/** Whether `event` completes `goal`; `boxes` is how many boxes were placed before it. */
export function reached(goal: Goal, event: BoardEvent, boxes: number): boolean {
  if ("flat" in event) {
    return goal === "flat" ? event.flat : goal === "turn" && !event.flat;
  }
  switch (goal) {
    case "place":
      return event.boxes > boxes;
    case "remove":
      return event.boxes < boxes;
    case "solve":
      return event.solved;
    default:
      return false;
  }
}

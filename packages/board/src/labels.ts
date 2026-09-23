/**
 * Clue label text size (CSS px) on a board `width` px wide: smaller on a phone, so the
 * labels leave the markers room.
 */
export function labelFont(width: number): number {
  return Math.min(14, Math.max(10, width / 32));
}

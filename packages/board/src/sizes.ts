/**
 * Clue label text size (CSS px) on a board `width` px wide: smaller on a phone, so the
 * labels leave the markers room.
 */
export function labelFont(width: number): number {
  return Math.min(14, Math.max(10, width / 32));
}

/**
 * Side (CSS px) of the view cube on a board `width` px wide: 48 on a desktop, smaller on
 * a phone, where a full-size one would sit on the cube's top-right corner.
 */
export function viewCubeSide(width: number): number {
  return Math.min(48, Math.max(28, width * 0.09));
}

/**
 * CSS `perspective` (px) that shows a view cube `side` px across as the camera shows a
 * board `size` cells across from `distance` cells away: the same foreshortening.
 */
export function viewCubePerspective(
  side: number,
  distance: number,
  size: number,
): number {
  return (side * distance) / size;
}

/**
 * How far (CSS px) the view cube's box sits from the board's corner. Turned, and larger
 * where it is nearer, the cube reaches past its box by up to about 0.43 of its side.
 */
export function viewCubeInset(side: number): number {
  return side * 0.5;
}

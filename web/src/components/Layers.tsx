import {
  boxIndexAt,
  clueAt,
  clueColor,
  clueIndexOf,
  clueText,
  type Puzzle,
} from "@/lib/puzzle";
import { ShapeIcon } from "@/components/ShapeIcon";

/** The cube as horizontal layers (y = 0 is the bottom). Columns are x, rows are z. */
export function Layers({
  puzzle,
  showSolution,
}: {
  puzzle: Puzzle;
  showSolution: boolean;
}) {
  const range = Array.from({ length: puzzle.size }, (_, i) => i);
  return (
    <div className="grid grid-cols-2 gap-6 sm:grid-cols-4">
      {range.map((y) => (
        <figure key={y}>
          <div className="grid grid-cols-4 gap-1">
            {range.map((z) =>
              range.map((x) => {
                const clue = clueAt(puzzle.clues, [x, y, z]);
                const box = showSolution
                  ? boxIndexAt(puzzle.solution, [x, y, z])
                  : -1;
                const ci =
                  box >= 0
                    ? clueIndexOf(puzzle.solution[box], puzzle.clues)
                    : clue
                      ? puzzle.clues.indexOf(clue)
                      : -1;
                return (
                  <div
                    key={`${x}-${z}`}
                    className="flex aspect-square flex-col items-center justify-center rounded border border-zinc-300 font-mono text-sm leading-none"
                    style={{ background: ci >= 0 ? clueColor(ci) : undefined }}
                  >
                    {clue?.shape && <ShapeIcon shape={clue.shape} />}
                    {clue ? clueText(clue) : ""}
                  </div>
                );
              }),
            )}
          </div>
          <figcaption className="mt-2 text-center text-sm text-zinc-500">
            Layer {y + 1}
          </figcaption>
        </figure>
      ))}
    </div>
  );
}

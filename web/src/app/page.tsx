import { GameModes } from "@/components/GameModes";
import { ShapeIcon } from "@/components/ShapeIcon";
import { puzzle, SHAPE_NAME } from "@/lib/puzzle";
import type { Shape } from "patches-board";

export default function Home() {
  return (
    <GameModes weekly={puzzle}>
      <div className="mt-16 grid gap-12 sm:grid-cols-2">
        <section>
          <h2 className="text-sm font-medium text-zinc-400">How to play</h2>
          <ul className="mt-4 space-y-2 text-base">
            <li>
              Drag from one cell to another to build the box between them, or
              click both cells.
            </li>
            <li>
              Pause on a cell mid-drag to keep it in the box, then drag on to
              grow the box another way.
            </li>
            <li>
              A box that breaks its clue shows as a red outline. Drag from it to
              grow it into the right shape.
            </li>
            <li>Click a box to remove it, or drag from it to extend it.</li>
            <li>Drag the space around the cube to turn it.</li>
            <li>
              Scroll or pinch to zoom. Zoom in far enough and the layer nearest
              you peels away.
            </li>
            <li>
              Keyboard: Tab to the board. Arrows move, Shift+↑/↓ changes layer,
              Space starts and places a box (on a box it grows it), Delete
              removes, Esc cancels, Shift+←/→ turns the cube, +/− zoom.
            </li>
          </ul>
        </section>
        <section>
          <h2 className="text-sm font-medium text-zinc-400">Clues</h2>
          <p className="mt-4 text-base">
            A clue&apos;s marker has its box&apos;s shape. The number is the
            box&apos;s volume; ? hides it.
          </p>
          <ul className="mt-4 grid grid-cols-2 gap-x-6 gap-y-2 text-base">
            {(Object.keys(SHAPE_NAME) as Shape[]).map((s) => (
              <li key={s} className="flex items-center gap-2">
                <ShapeIcon shape={s} />
                {SHAPE_NAME[s]}
              </li>
            ))}
            <li className="flex items-center gap-2">
              <ShapeIcon />
              any shape
            </li>
          </ul>
        </section>
      </div>
    </GameModes>
  );
}

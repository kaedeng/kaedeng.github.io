import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";
import { ShapeIcon } from "@/components/ShapeIcon";
import { puzzle, SHAPE_NAME, type Shape } from "@/lib/puzzle";

export default function Home() {
  return (
    <>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        Puzzle {puzzle.id}
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Fill the 4×4×4 cube with boxes. Every box holds exactly one clue and
        takes that clue&apos;s shape and volume.
      </p>
      <div className="mt-12">
        <Cube puzzle={puzzle} mode="play" />
      </div>
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
            <li>Click a box to remove it, or drag from it to extend it.</li>
            <li>Drag the space around the cube to turn it.</li>
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
      <details className="mt-16">
        <summary className="cursor-pointer text-sm text-zinc-400 hover:text-white">
          Clues layer by layer (layer 1 is the bottom)
        </summary>
        <div className="mt-6">
          <Layers puzzle={puzzle} showSolution={false} />
        </div>
      </details>
    </>
  );
}

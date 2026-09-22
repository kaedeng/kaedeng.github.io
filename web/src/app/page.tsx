import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";
import { puzzle } from "@/lib/puzzle";

export default function Home() {
  return (
    <>
      <h1 className="text-4xl font-bold tracking-tight">Puzzle {puzzle.id}</h1>
      <p className="mt-4 max-w-prose text-lg leading-relaxed">
        Fill the whole 4×4×4 cube with boxes. Every box must contain exactly one
        clue. A clue&apos;s marker has the shape of its box: a cube, a tall or
        flat block, a bar or a wall. A round marker means any shape. A number is
        the box&apos;s volume; ? means you work it out. Drag from one cell to
        another to build the box between them, or click both cells. Click a box
        to remove it, or drag from it to rebuild it. Drag empty space to orbit.
      </p>
      <div className="mt-8">
        <Cube puzzle={puzzle} mode="play" />
      </div>
      <details className="mt-8">
        <summary className="cursor-pointer text-sm text-zinc-500">
          Clues layer by layer (layer 1 is the bottom)
        </summary>
        <div className="mt-4">
          <Layers puzzle={puzzle} showSolution={false} />
        </div>
      </details>
    </>
  );
}

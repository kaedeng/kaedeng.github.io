import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";
import { puzzle } from "@/lib/puzzle";

export default function Home() {
  return (
    <>
      <h1 className="text-4xl font-bold tracking-tight">Puzzle {puzzle.id}</h1>
      <p className="mt-4 max-w-prose text-lg leading-relaxed">
        Fill the whole 4×4×4 cube with boxes. Every box must contain exactly one
        clue cell. A numbered clue is the volume of its box; a ? clue means you
        work out the size yourself. Click a cell, then a second cell, to place
        the box that spans them. Click a placed box to remove it. Drag to orbit.
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

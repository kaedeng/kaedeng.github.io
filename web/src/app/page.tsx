import { Layers } from "@/components/Layers";
import { puzzle } from "@/lib/puzzle";

export default function Home() {
  return (
    <>
      <h1 className="text-4xl font-bold tracking-tight">Puzzle {puzzle.id}</h1>
      <p className="mt-4 max-w-prose text-lg leading-relaxed">
        Fill the whole 4×4×4 cube with boxes. Every box must contain exactly one
        clue cell. A numbered clue is the volume of its box; a ? clue means you
        work out the size yourself. The 3D board is coming soon; until then,
        here are the clues layer by layer.
      </p>
      <div className="mt-8">
        <Layers puzzle={puzzle} showSolution={false} />
      </div>
    </>
  );
}

import { Cube } from "@/components/Cube";
import { Layers } from "@/components/Layers";
import { puzzle } from "@/lib/puzzle";

export default function Answer() {
  return (
    <>
      <h1 className="text-4xl font-semibold tracking-tighter sm:text-6xl">
        Answer for {puzzle.id}
      </h1>
      <p className="mt-6 max-w-prose text-xl text-zinc-400">
        Each colour is one box. Drag the cube to look around it. Below it, the
        same solution layer by layer: layer 1 is the bottom, columns run along x
        and rows along z.
      </p>
      <div className="mt-12">
        <Cube puzzle={puzzle} mode="answer" />
      </div>
      <div className="mt-16">
        <Layers puzzle={puzzle} showSolution />
      </div>
    </>
  );
}

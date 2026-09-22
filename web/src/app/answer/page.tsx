import { Layers } from "@/components/Layers";
import { puzzle } from "@/lib/puzzle";

export default function Answer() {
  return (
    <>
      <h1 className="text-4xl font-bold tracking-tight">
        Answer for {puzzle.id}
      </h1>
      <p className="mt-4 max-w-prose text-lg leading-relaxed">
        Each colour is one box. Layer 1 is the bottom of the cube; columns run
        along x and rows along z. The 3D view of the solution is coming soon.
      </p>
      <div className="mt-8">
        <Layers puzzle={puzzle} showSolution />
      </div>
    </>
  );
}

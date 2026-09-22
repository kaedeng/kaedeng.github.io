export default function Home() {
  return (
    <>
      <h1 className="text-4xl font-bold tracking-tight">Weekly puzzle</h1>
      <p className="mt-4 max-w-prose text-lg leading-relaxed">
        Fill the whole 4×4×4 cube with boxes. Every box must contain exactly one
        clue cell. A numbered clue is the volume of its box; a ? clue means you
        work out the size yourself. Click a cell, then a second cell, to place
        the box that spans them. Click a placed box to remove it. Drag to orbit.
      </p>
      <div className="mt-8 flex aspect-square w-full items-center justify-center rounded border border-zinc-300 text-zinc-500">
        Puzzle coming soon
      </div>
    </>
  );
}

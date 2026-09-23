# patches-board

The 3D Patches board for any site: the Rust renderer in `crates/wasm` compiled to WebAssembly, plus `mountBoard()`, a small framework-free wrapper that loads it and wires it to a canvas.

## Build

```bash
pnpm build   # wasm-pack: crates/wasm -> wasm/, then tsc: src/ -> dist/
```

Needs Rust with the `wasm32-unknown-unknown` target, and wasm-pack.

## Use

```html
<div style="position: relative; width: 600px; height: 600px">
  <canvas
    tabindex="0"
    style="display: block; width: 100%; height: 100%; touch-action: none"
  ></canvas>
</div>
```

```js
import { mountBoard } from "patches-board";

const board = await mountBoard(canvas, puzzle, {
  onStatus: ({ boxes, wrong, solved }) => {}, // on mount and after every change
  onPlay: () => {}, // every press on a cell or board key, e.g. to start a timer
});
board.reset();
board.destroy();
```

- `puzzle` is the JSON printed by `cargo run -p patches-core --bin generate -- <seed>` (type `Puzzle`).
- `answer: true` shows the stored solution; the cube still turns and zooms.
- The canvas should fill a positioned parent. Clue labels, the vim mode line, the view cube (top right; a face shows that layer in 2D) and the 2D layer bar go in an overlay added next to the canvas, and take the parent's font.
- The drawing buffer is sized once, at mount: the canvas's CSS size times `devicePixelRatio`.
- The `.wasm` is fetched from next to `wasm/patches_wasm.js` (`import.meta.url`). That works as plain ES modules with no bundler, under any base path, and with bundlers that handle `new URL(..., import.meta.url)` (Vite, webpack 5, Turbopack).
- `generatePuzzle(seed)` resolves to the same puzzle as the `generate` CLI for that seed, generated in the browser.
- `PALETTE`, `clueColor`, `clueText` and the puzzle types are exported too, for UI around the board.

## Moving it to another site

- `pnpm build && pnpm pack` here gives `patches-board-0.1.0.tgz` with `dist/` and `wasm/` built in. `pnpm add ./patches-board-0.1.0.tgz` there; that site needs no Rust. Puzzles still come from the Rust generator, so commit its JSON or run it in that site's build.
- Or copy `crates/`, the root `Cargo.toml` and `packages/board/` into the other repo and build there.

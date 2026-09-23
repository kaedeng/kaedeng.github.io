# Decision log

Your methods section. About one page total.

Answer these as you go, not the night before it is due.
Specifics beat polish - a short honest answer is worth more than a long vague one.

Delete these instructions when you are done, or leave them. It does not matter.

---

## 1. What did you set out to build, and what changed?

What you wanted at the start, and what is actually live now.
Name one thing you dropped or added along the way, and why.

I wanted 3D patches (like the LinkedIn game). I got 3D patches out of it. One thing i added along the way was keyboard interaction, since normal 2D patches is so easy to interact with using mouse, but 3D patches is more finnicky, so I added keyboard support for a simplified experience.

---

## 2. A fork in the road

Name one real choice where you could have gone two ways.
Plain HTML or a framework. One page or several. Your own CSS or someone's template.
What goes on the front page and what does not.

Say which you picked, what the alternative was, and what you gave up by not taking it.

"There was no alternative" is not an answer. Find the fork.

Plain HTML vs framework. I chose framework since the site is mostly one interactive 3D board (Rust compiled to WebAssembly) plus two pages that share the same pieces (the board, the layer grids, the header), and Next.js with Tailwind let me build those once as components instead of copying markup between pages. The alternative was the plain HTML and CSS template the course recommends. What I gave up: no build step and an instant deploy. The README warned that Next.js static export tends to eat time on toolchain problems, and it did cost setup: Pages had to switch from "deploy from a branch" to a GitHub Actions build, and the wasm file has to be loaded at runtime, outside Next's bundler, to work in the static export.

---

## 3. Where you overruled the agent

One time Claude suggested, wrote, or claimed something and you did not take it.

What did it do? How did you notice? What did you do instead?

If it genuinely never happened, say so plainly, and then say what you would have had to
check in order to notice. Being honest here costs you far less than a story you cannot
defend when you record your video.

Originally, Claude implemented the click-drag feature to overwrite the existing box, but I thought that was bad ux since the feel of it was clunky when I manually tried it, so I decided to tell Claude to make it add on to the existing box, and to only clear the box on tapping once.

---

## 4. How you know it works

What check did you run, and what did it tell you?

Then the real question: **what would have made this check fail?**
A check that could not have failed is not a check.

Link to your `verification/` folder.

[verifications folder](./verification/)

The check is fetching the live URL (`curl -s https://kaedeng.github.io/`, saved as `verification/fetch.txt`) and looking for the page heading `Puzzle 2026-W39`, which only exists in the page Next.js builds; the screenshot shows the same page in a browser with the URL bar visible. It would have failed if GitHub Pages were still set to deploy from the main branch instead of the Actions build: the URL would have shown the repo's README, not the game. On top of that, every push runs the Rust tests (the puzzle rules, the generator, picking and keyboard input), lint and a full build in CI, and the site only deploys if they all pass.

---

## 5. What is still wrong

One thing on your own site that is not right, not finished, or that you do not
fully understand.

What would you do next, and how would you find out?

I would love to fix the click-drag and general mouse/touch interaction, it works as is but I don't know the idea proper ux way to do this, so I don't know how to tell Claude to fix it.

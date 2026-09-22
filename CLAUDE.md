# CLAUDE.md

- State assumptions before implementing. If something is unclear or has more than one reasonable reading, ask instead of picking silently.
- Say so when a simpler approach exists; push back when warranted.
- Test-driven development: write the failing test first, then make it pass.
- Cyclomatic complexity < 10 per function.
- Build only what was asked: no speculative features, abstractions for single-use code, configurability, or error handling for impossible cases.
- Touch only what the request requires. Don't refactor or reformat adjacent code; match the existing style. Mention unrelated dead code, don't delete it. Remove only the imports/variables/functions your own change orphaned.
- For multi-step tasks, state a short plan with a verification check per step, and loop until the checks pass.

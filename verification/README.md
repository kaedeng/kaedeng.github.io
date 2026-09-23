URL checked: https://kaedeng.github.io
When: 2026-09-23 17:15
What would have made this fail: fetch.txt would hold the repo's README instead of the exported game page (no "Daily" page heading, whose date the page's script fills in) if GitHub Pages deployed from the main branch rather than from the Actions build of web/out.

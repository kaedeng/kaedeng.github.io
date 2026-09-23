# Build the site and serve it: `just`, or `just serve 3000` for another port.
default: serve

# Build the wasm board and the static site into web/out.
build:
    pnpm install --frozen-lockfile
    cd web && pnpm wasm && pnpm build

# Build, then serve web/out.
serve port="8000": build
    @echo "Serving on http://localhost:{{port}} (Ctrl+C stops it)"
    python3 -m http.server {{port}} -d web/out

# Next's dev server, which reloads on web changes. Rust changes need a restart.
dev:
    pnpm install --frozen-lockfile
    cd web && pnpm wasm && pnpm dev

# Everything CI checks before a deploy.
check:
    cargo test --workspace
    cargo clippy -p patches-core --all-targets -- -D warnings
    cargo clippy -p patches-wasm --target wasm32-unknown-unknown -- -D warnings
    cd web && pnpm lint && pnpm format:check && pnpm test

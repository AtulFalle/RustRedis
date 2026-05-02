# AGENTS.md

## Commands
- `cargo build --verbose` runs in CI for repository builds.
- `cargo test --verbose` runs in CI for repository tests.
- `cargo run` starts the server locally; `src/main.rs` binds to `127.0.0.1:6379`.

## Workflow Notes
- GitHub Actions workflow: `.github/workflows/rust.yml`
- CI triggers on pushes and pull requests targeting `master`.

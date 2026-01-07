# Contributing

## Prerequisites

- Rust (stable) and Cargo installed.
- `rustfmt` and `clippy` are recommended for code style checks:

```powershell
rustup component add rustfmt clippy
```

## Build & run

From the repository root:

```powershell
cargo build --release
cargo run
```

## Code style

- Run `cargo fmt` before submitting a PR.
- Fix warnings or run `cargo clippy -- -D warnings` to catch common issues.

## Testing

- If crates or modules include tests, run:

```powershell
cargo test
```

## Branching and PRs

- Fork the repository and create a feature branch from `main`.
- Keep changes focused and small; one logical change per PR.
- Include a brief description of the problem, the approach, and any notable design decisions in the PR description.

## Commit messages

- Use clear, present-tense messages. Example: "Add Perlin terrain generator".

## Documentation

- Update `docs/` for any behavioral or API changes.
- Keep `README.md` concise; place long-form history or design notes under `docs/`.

## Review checklist (maintainers)

- Builds cleanly: `cargo build` passes.
- No warnings or lints flagged by `clippy` (or documented rationale).
- Adequate tests or manual verification steps included for complex changes.

# Contributing to Terminalist

Thanks for taking the time to contribute!

## Development setup

- Rust 1.78+ (MSRV pinned)
- Install components: `rustup component add rustfmt clippy`

## Workflow

1. Create a feature branch
2. Run format and lint: `cargo fmt && cargo clippy -- -D warnings`
3. Run tests: `cargo test`
4. Build locally: `cargo build --release`
5. Open a PR with a clear title and description

## Commit style

- Use clear, conventional titles when possible
  - feat(...):, fix(...):, docs(...):, chore(...):, refactor(...):
- Keep changes focused and small

## PR checklist

- [ ] Code formatted (`cargo fmt`)
- [ ] Lints pass (`cargo clippy -- -D warnings`)
- [ ] Tests pass (`cargo test`)
- [ ] Updated docs/README if behavior or flags changed

## Running

- Show help: `cargo run -- --help`
- Show version: `cargo run -- --version`
- Debug DB mode: `cargo run -- --debug`

## Reporting issues

Please include:
- Repro steps
- Expected vs actual behavior
- OS and terminal emulator
- `rustc --version`

---

By contributing, you agree that your contributions will be licensed under the MIT License.


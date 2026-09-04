# Contributing to Terminalist

Thanks for taking the time to contribute!

## Development setup

- Rust 1.94+ (MSRV pinned)
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

## Versioning development builds

Use a SemVer prerelease suffix when a development build needs a distinct version. For
example, builds leading up to `0.6.0` can use:

```text
0.6.0-dev.1
0.6.0-dev.2
```

Increment the final number when publishing or sharing another development build that needs to
be identified separately. Ordinary feature and bug-fix changes do not need their own version
bump.

When changing the package version, update `Cargo.toml` and run `cargo check` to synchronize
`Cargo.lock`. Commit both files together. Release-preparation changes should replace the
development suffix with the final release version, such as `0.6.0`.

## Reporting issues

Please include:
- Repro steps
- Expected vs actual behavior
- OS and terminal emulator
- `rustc --version`

---

By contributing, you agree that your contributions will be licensed under the MIT License.

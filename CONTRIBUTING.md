# Contributing to Sayzo

Thanks for helping improve Sayzo!

## Reporting bugs

Open an [issue](https://github.com/op-h/sayzo/issues/new/choose) and include:
- the app you were reading from (browser, PDF reader, terminal, and so on)
- your Sayzo version (shown in the dashboard sidebar) and Windows version
- what you expected and what happened instead

## Development setup

See [Build from source](README.md#build-from-source) in the README. In short:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/download-models.ps1
npm install
npm run tauri dev
```

## Pull requests

1. Fork the repo and create a branch from `main`.
2. Keep changes focused: one fix or feature per pull request.
3. Make sure these pass before opening the PR:
   ```powershell
   npm run build                      # TypeScript check + frontend build
   cd src-tauri; cargo test           # Rust unit tests
   cargo test -- --ignored            # voice tests (needs the models)
   ```
4. Describe what changed and how you tested it.

## Code style

- **Rust:** `cargo fmt` and `cargo clippy`.
- **TypeScript/React:** follow the style of the surrounding code. UI primitives live in `src/components/ui`.
- Don't commit the voice models or build output (they're in `.gitignore`).

By contributing, you agree that your contributions are licensed under the GPL-3.0.

# Contributing to DockForge

Thanks for helping! Two ways to contribute:

- **Themes** go to [dockforge-themes](https://github.com/dockforge/dockforge-themes), not here.
- **Code**: open an issue first for anything bigger than a small fix, so we can agree on the approach.

## Before opening a pull request

```powershell
npm run check                                        # Svelte + TypeScript
cargo test --manifest-path src-tauri/Cargo.toml      # Rust tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## Ground rules

- No code injection into Explorer or other processes. The dock stays an ordinary window.
- The web side gets no new powers (file system, shell, HTTP) without a design discussion: themes run there.
- Keep idle memory low; the dock runs all day.

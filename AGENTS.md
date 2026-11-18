# Repository guide for coding agents

Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) when changing behavior or adding a feature. Read [DESIGN.md](docs/DESIGN.md) when changing the interface.

For each code change, run `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets -- -D warnings`, and `cargo test --locked`. When changing `cfg`-gated or platform-specific code, also run `scripts/check-platforms.sh`, which lints the macOS, Linux and Windows targets. Record the OS and manual steps for visible behavior; automated tests do not validate the desktop appearance.

## Issues

GitHub Issues are the primary way this repo tracks work: features, UX fixes, and bugs. Check them before starting, work from one, and file a new issue for anything you find but do not fix.

Branch per issue off `main` (for example `issue-<n>-short-name`), and end the commit message body with `Refs #<n>`. When the work lands, comment with the fixing commit and close the issue.

## Commit messages

Use Conventional Commits: a `type(scope): summary` subject in the imperative, under 72 characters. Types are `feat`, `fix`, `refactor`, `perf`, `test`, `docs`, `build`, and `chore`; the scope names the area touched (`preview`, `nav`, `gutters`, `menu`, `find`, `theme`, `agents`). The body is a one- or two-sentence summary of what the change does for the user, then a succinct bullet list of the notable changes. Close with `Refs #<n>` when there is an issue. Never add a co-author trailer or any agent-attribution line (global rule; no repository may override it).

```text
feat(preview): select and copy rendered Markdown text

Drag across the rendered preview to highlight text and copy it with
Ctrl/Cmd+C, the way selecting text on a web page works.

- Add a pure selection model in domain/selection.rs with range tests.
- Copy with Ctrl/Cmd+C; clear with Escape.

Refs #26
```

## Changelog

`CHANGELOG.md` is generated from the Conventional Commit subjects by [git-cliff](https://git-cliff.org) using `cliff.toml`; do not edit it by hand. Regenerate it with `git cliff -o CHANGELOG.md` when cutting a release (tag first, or pass `--tag vX.Y.Z`). Commits that are not Conventional Commits, and `docs`, `test`, `build` and `chore` commits, are left out.

## Code layout

Keep filesystem and folder rules in their existing modules. Add a `Message` in `src/app/message.rs` for new user actions, and handle it in the matching `src/app/update/` module (route it in `update/mod.rs`). Keep `src/app/view/` free of state transitions; keep `src/domain/` free of `iced` widgets. Keep the `iced` views focused on rendering current state.

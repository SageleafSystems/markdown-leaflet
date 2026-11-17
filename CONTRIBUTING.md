# Contributing

Thanks for helping improve Markdown Leaflet. For a bug or feature request, open a GitHub issue with the operating system, the behavior you saw, and a small Markdown example when relevant. Remove private content from examples and screenshots.

## Development

Install Rust and Cargo. From the repository root:

```sh
cargo run --locked -- path/to/a/markdown/folder
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
```

To install a user-local release build for use outside the repository, run `./scripts/install.sh`. It installs to `~/.local/bin`; run `./scripts/install.sh remove` to remove that copy. Set `PREFIX` to use a different install prefix.

Use `cargo fmt --all` before submitting. The CI workflow runs the same checks. If you change a visible flow, describe the manual steps you tried and the OS in your pull request; [docs/MANUAL_TEST.md](docs/MANUAL_TEST.md) is the release checklist. A small focused change with an example of the behavior is easiest to review.

The code map is in [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). Folder navigation and search are a useful first area to explore: [src/domain/folder.rs](src/domain/folder.rs) contains the rules and tests, while [src/app/](src/app/) connects them to the interface.

## License and sign-off

Markdown Leaflet is licensed under [GPL-3.0-or-later](LICENSE), copyright Sageleaf Systems. By contributing you agree that your work is released under the same license. There is no CLA; instead, certify the [Developer Certificate of Origin](https://developercertificate.org/) by signing off each commit:

```sh
git commit -s -m "Describe the change"
```

This adds a `Signed-off-by: Your Name <you@example.com>` line using your Git identity. Pull requests with unsigned commits will be asked to amend them. New `.rs` files start with `// SPDX-License-Identifier: GPL-3.0-or-later`. Dependencies must pass `cargo deny check licenses`.

Community support is currently best effort. Please keep reports and reviews specific and respectful.

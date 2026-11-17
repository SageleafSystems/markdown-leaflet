# CI roadmap

`.github/workflows/ci.yml` defines more jobs than run by default during the
v0.1.0-alpha stage. This records the full picture so none of it gets lost.

## Why trim it now

CI only just started running on the public GitHub repo (previously
unverified — see the original `.gitea`/GitHub Actions setup issue). The
first batch of runs queued for an extended period with nothing executing,
consistent with Actions not yet fully enabled/configured for this org
(check Settings → Actions → General at the org and repo level, and any
spending limit, before assuming a job itself is broken). Running the full,
heavier matrix while that's still being sorted out would only produce
noisy, hard-to-diagnose failures. Trim to the jobs that give the most
signal per run, confirm those go green, then bring the rest back one at a
time.

## Active during alpha

| Job | What it checks |
| --- | --- |
| `check` (`ubuntu-latest`, `macos-latest`) | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo build --release` on the two platforms this app actually supports and ships on. |

## Paused (`if: false` in the workflow, not deleted)

| Job | What it checks | Re-enable when |
| --- | --- | --- |
| `check` (`windows-latest`) | Same checks as above, on an unsupported platform. Was already `continue-on-error` (informational only). | Windows becomes an actual supported target; add it back to the `check` matrix rather than running it separately. |
| `msrv` | Builds with the exact MSRV toolchain (`1.93.0`) pinned in `Cargo.toml`, so a newer language/library feature can't silently slip in. | Once the essential matrix is confirmed green on the public forge; this is cheap and worth restoring early. |
| `deny` | `cargo-deny` advisories/licenses/sources scan. | Same — cheap, worth restoring early, especially after a batch of dependency bumps. |
| `docs` | `cargo doc --document-private-items` with warnings as errors. | Any time; low priority, not release-blocking. |
| `bundle-macos` | Builds and verifies the signed-ad-hoc `.app`/`.dmg` in CI. | When release packaging is automated for real (tracked separately: signing, notarization, universal binary, tag-triggered release workflow). Until then this duplicates the manual build done for each alpha release. |
| `bundle-linux` | Builds the AppImage and smoke-tests it under Xvfb. | Same as `bundle-macos` — once release packaging is automated, or once Linux distribution is prioritized again. |

## Restoring a paused job

Delete its `if: false` line (and the comment above it). No other change
needed — the job bodies were left intact on purpose.

# An ordinary document

This file has no frontmatter block at all — it's the control case. The Details panel should
show FILE and CONTENT exactly as before, with no FRONTMATTER section in between, and the window
title should read this file's own name, `no-frontmatter.md`.

A document that merely *contains* a `---` further down, like the one below, is not frontmatter
either — detection only ever looks at the very first line of the file, so the thematic break
below renders exactly as it always has.

---

Text after an ordinary thematic break.

---
title: This Never Closes

# An opening delimiter with no closing one

This file starts with a bare `---` but there is no second `---` (or `...`) anywhere below it, so
by rule it is **not** frontmatter — detection requires both delimiters. Expect the opening
`---` to render as an ordinary thematic break, `title: This Never Closes` as a plain paragraph,
and no FRONTMATTER section in the Details panel: identical to how this file rendered before
issue #65.

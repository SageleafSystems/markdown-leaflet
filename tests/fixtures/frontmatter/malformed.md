---
title: Malformed Example
tags: [one, two
---

# A frontmatter block that fails to parse

The block above opens a `tags:` sequence with `[` but never closes it, so it isn't valid YAML.
Expect:

- **No FRONTMATTER section** in the Details panel for this document.
- The block renders exactly as it would with no frontmatter support at all: the two `---` lines
  as thematic breaks, and `title: Malformed Example` / `tags: [one, two` as an ordinary
  paragraph, right above this heading.
- The footer's status area shows a small warning glyph; hover it for why the frontmatter
  couldn't be parsed.

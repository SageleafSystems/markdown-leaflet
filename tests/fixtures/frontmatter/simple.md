---
title: A Simple Frontmatter Example
date: 2026-01-15
tags: [one, two, three]
draft: false
---

# Frontmatter demo

This file opens with an ordinary YAML frontmatter block, the kind Jekyll, Hugo, Obsidian and
Zettlr all write. Open it and check three things:

- The **window title** and the toolbar's document title both read "A Simple Frontmatter
  Example", not `simple.md` — the frontmatter `title` field overrides the file name (issue #65).
- The right gutter's **Details** tab shows a FRONTMATTER section above FILE, with `title`,
  `date`, `tags`, and `draft` listed in that order, matching the order they're written above.
- The block itself (the `---` lines and everything between them) does **not** appear in this
  rendered preview — it should start cleanly at the heading below.

## Below the frontmatter

Only this part of the file counts toward the Content stats (words, characters, lines) in the
Details panel — the frontmatter block is metadata, not prose.

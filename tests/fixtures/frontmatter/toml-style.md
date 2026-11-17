+++
title = "A TOML-style block"
tags = ["one", "two"]
+++

# Not recognized, by design

The block above uses `+++` delimiters, the TOML-frontmatter convention some Hugo sites use
instead of YAML. Markdown Leaflet only understands `---`/`---` YAML blocks (issue #65, v1
scope) — a `+++` line isn't even a valid CommonMark thematic break (only `-`, `*`, and `_`
qualify), so it renders as an ordinary paragraph, and no FRONTMATTER section appears in the
Details panel.

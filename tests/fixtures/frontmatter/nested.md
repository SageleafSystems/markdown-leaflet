---
title: Nested Frontmatter Values
tags: [rust, iced, desktop]
author:
  name: Ada Lovelace
  email: ada@example.com
contributors:
  - name: Grace Hopper
    role: reviewer
  - name: Alan Turing
    role: editor
---

# Nested values in the Details panel

This file's frontmatter mixes three shapes, to exercise the recursive rendering in the
FRONTMATTER section:

- `tags` is a flat list of scalars — it should render as one row, `rust, iced, desktop`.
- `author` is a map — it should render as a row labeled `author {2}`, followed by two indented
  rows, `name` and `email`, one guide line deeper (the same nesting the Outline tab uses for
  headings).
- `contributors` is a list of maps — it should render as a row labeled `contributors [2]`,
  followed by two indented group rows, `[0] {2}` and `[1] {2}`, each with its own `name` and
  `role` nested one level deeper still.

---
name: pdf-forms
description: >
  Fills PDF forms from structured data. Use when the user asks to
  fill or inspect a PDF form.
license: Complete terms in LICENSE.txt
allowed-tools: Read Bash(python:*)
metadata:
  author: Acme Docs Team
  version: "1.2"
---

# PDF forms

1. Inspect the form with `python scripts/fill.py --list form.pdf`.
2. Fill it: `python scripts/fill.py form.pdf data.json`.

See [reference.md](reference.md) for field types.

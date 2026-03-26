# arxiv2md

`arxiv2md` is a Rust CLI that converts arXiv papers into Markdown optimized for LLM agents.

It resolves papers using this fallback chain:

1. `arxiv.org/html/<id>`
2. `ar5iv.labs.arxiv.org/html/<id>`
3. local LaTeX conversion from `arxiv.org/e-print/<id>` via `pandoc`
4. PDF text extraction from `arxiv.org/pdf/<id>.pdf`

## Why this tool

- Markdown-first output
- stdout by default
- low-noise defaults for agent workflows
- support for old and new arXiv identifiers
- cached fetch artifacts for repeated runs

## Install

```bash
cargo install --path .
```

For full source-fallback coverage, install `pandoc` and ensure it is on `PATH`.

## Usage

```bash
arxiv2md 2501.11120
arxiv2md 1706.03762 --frontmatter -o paper.md
arxiv2md hep-th/9901001 --keep-refs --keep-toc
arxiv2md 2501.11120 --section-filter-mode include --sections "Abstract,Introduction"
```

## Flags

- `-o, --output <PATH>`: output file, default is stdout. Use `-` for stdout explicitly.
- `--frontmatter`: prepend YAML metadata.
- `--keep-refs`: keep references/bibliography sections.
- `--keep-toc`: include a generated table of contents.
- `--remove-inline-citations`: remove inline citations when supported by the resolved format.
- `--section-filter-mode <include|exclude>` with `--sections` or repeatable `--section`
- `--include-tree`: include a plain section tree before the body when section structure is available.
- `--refresh`: bypass the local artifact cache.

## Notes

- HTML and ar5iv paths produce the cleanest results.
- If `pandoc` is unavailable and the tool reaches the LaTeX fallback stage, it warns and falls through to PDF extraction.
- PDF fallback is intentionally best-effort and may not preserve section structure.

# arxiv2md

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Cargo](https://img.shields.io/badge/rust-2024-orange.svg)](https://www.rust-lang.org/)

`arxiv2md` is a Rust-based CLI tool designed to convert arXiv papers into clean, high-signal Markdown optimized for Large Language Model (LLM) agents and research workflows. It handles everything from the latest HTML representations to legacy LaTeX source archives and PDF extraction.

## Key Features

- **LLM-Ready Markdown:** Generates clean, low-noise Markdown suitable for context windows.
- **Robust Fallback Chain:** Resolves papers through four different stages to ensure the best possible extraction quality.
- **Section Filtering:** Include or exclude specific sections (e.g., Abstract, Introduction, Conclusion) to focus on relevant content.
- **Metadata Support:** Optional YAML frontmatter generation using the arXiv Atom API.
- **Smart Caching:** Persists fetched artifacts (HTML, PDF, source, metadata) locally to avoid redundant network requests.
- **Citation Management:** Optionally remove inline citations (e.g., `[1, 2]`) for even cleaner text.

## Core Fallback Strategy

`arxiv2md` attempts to resolve papers using the following prioritized chain:

1.  **arXiv HTML:** Direct extraction from the official HTML representation (`arxiv.org/html/<id>`).
2.  **ar5iv:** Extraction from the ar5iv labs representation (`ar5iv.labs.arxiv.org/html/<id>`).
3.  **LaTeX Source:** Download of the source archive (`arxiv.org/e-print/<id>`) and conversion via `pandoc`.
4.  **PDF Extraction:** Best-effort text extraction from the official PDF (`arxiv.org/pdf/<id>.pdf`).

## Prerequisites

For full functionality, especially the **LaTeX Source fallback**, you must have [Pandoc](https://pandoc.org/) installed and available on your system's `PATH`.

## Installation

### From Source (using Cargo)

Ensure you have the Rust toolchain installed (2024 edition supported).

```bash
git clone https://github.com/alexferrari88/arxiv2md
cd arxiv2md
cargo install --path .
```

### Pre-built Binaries

Download the latest pre-built binaries for Windows, Linux, and macOS from the [Releases](https://github.com/alexferrari88/arxiv2md/releases) page.

## Usage

### Basic Usage

Convert a paper and output to stdout:

```bash
arxiv2md 2501.11120
```

### Advanced Examples

Save to a file with YAML frontmatter:
```bash
arxiv2md 1706.03762 --frontmatter -o transformer.md
```

Include only specific sections:
```bash
arxiv2md 2501.11120 --section-filter-mode include --sections "Abstract,Introduction"
```

Keep references and table of contents:
```bash
arxiv2md hep-th/9901001 --keep-refs --keep-toc
```

Remove inline citations for a cleaner reading experience:
```bash
arxiv2md 2501.11120 --remove-inline-citations
```

### Command-Line Options

| Flag | Description |
| :--- | :--- |
| `-o, --output <PATH>` | Output file path (defaults to stdout). |
| `--frontmatter` | Prepend YAML metadata to the output. |
| `--keep-refs` | Keep references/bibliography sections. |
| `--keep-toc` | Include a generated table of contents. |
| `--remove-inline-citations` | Remove inline citations (e.g., `[1]`) when supported. |
| `--section-filter-mode` | Set to `include` or `exclude` for section filtering. |
| `--sections <LIST>` | Comma-separated list of sections to include/exclude. |
| `--include-tree` | Prepend a plain text section tree before the body. |
| `--refresh` | Bypass the local artifact cache and fetch fresh data. |

## Caching

`arxiv2md` caches all fetched artifacts (HTML, PDF, source archives, and metadata) in your system's standard cache directory. This ensures that subsequent runs for the same paper are near-instant and respect arXiv's rate limits.

- **Linux:** `~/.cache/arxiv2md`
- **macOS:** `~/Library/Caches/arxiv2md`
- **Windows:** `%LOCALAPPDATA%\arxiv2md\cache`

## Development

### Building and Testing

```bash
# Build the project
cargo build

# Run unit and integration tests
cargo test

# Check for linting issues
cargo clippy

# Format the codebase
cargo fmt
```

## License

This project is licensed under the [MIT License](LICENSE).

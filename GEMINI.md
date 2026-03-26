# GEMINI.md - arxiv2md

## Project Overview
`arxiv2md` is a Rust CLI tool designed to convert arXiv papers into Markdown optimized for Large Language Model (LLM) agents. It provides a high-signal, low-noise representation of academic papers by resolving them through a multi-stage fallback chain.

### Core Fallback Strategy
1.  **arXiv HTML (`arxiv.org/html/<id>`):** Direct extraction from the official HTML representation.
2.  **ar5iv (`ar5iv.labs.arxiv.org/html/<id>`):** Extraction from the ar5iv labs representation.
3.  **LaTeX Source (`arxiv.org/e-print/<id>`):** Download of the source archive and conversion via `pandoc`.
4.  **PDF Extraction (`arxiv.org/pdf/<id>.pdf`):** Best-effort text extraction from the PDF.

### Key Features
- **Markdown-first:** Generates clean Markdown suitable for LLM context windows.
- **Section Filtering:** Allows including or excluding specific sections (e.g., Abstract, Introduction).
- **Metadata Support:** Optional YAML frontmatter generation.
- **Caching:** Persists fetched artifacts (HTML, PDF, source, metadata) to avoid redundant network requests.
- **Citation Management:** Option to remove inline citations when using HTML-based sources.

## Tech Stack
- **Language:** Rust (2024 edition).
- **CLI Framework:** `clap`.
- **HTTP Client:** `reqwest`.
- **Parsing:** `scraper` (HTML), `roxmltree` (Atom API), `pdf-extract` (PDF).
- **Conversion:** `pandoc` (required for LaTeX fallback).
- **Serialization:** `serde`, `serde_yaml`.

## Commands

### Building and Running
- **Build:** `cargo build`
- **Run:** `cargo run -- <ID>` (e.g., `cargo run -- 2501.11120`)
- **Install:** `cargo install --path .`

### Testing and Validation
- **Run Tests:** `cargo test`
- **Linting:** `cargo clippy`
- **Formatting:** `cargo fmt`

## Project Structure
- `src/main.rs`: Application entry point.
- `src/lib.rs`: Library module declarations.
- `src/pipeline.rs`: Main coordination logic for the fallback chain.
- `src/cli.rs`: Command-line argument definitions and parsing.
- `src/id.rs`: Parsing and normalization of arXiv identifiers and URLs.
- `src/cache.rs`: Local artifact caching implementation.
- `src/html.rs`: Logic for parsing arXiv/ar5iv HTML.
- `src/latex.rs`: Handling of LaTeX source archives and `pandoc` invocation.
- `src/pdf.rs`: Text extraction from PDF files.
- `src/metadata.rs`: Parsing of arXiv metadata from the Atom API.
- `src/markdown.rs`: Rendering logic for the final Markdown output.
- `src/model.rs`: Core data structures and enums.
- `tests/`: Integration tests using `assert_cmd`.

## Development Conventions
- **Error Handling:** Centralized error management in `src/error.rs` using the `thiserror` crate.
- **Testing:** Integration tests are preferred for verifying the CLI behavior.
- **External Dependencies:** `pandoc` must be available on the `PATH` for full functionality (LaTeX fallback).
- **Metadata:** Paper metadata is fetched via the arXiv Atom API before attempting content resolution.

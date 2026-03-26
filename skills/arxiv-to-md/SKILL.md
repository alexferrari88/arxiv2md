---
name: arxiv-to-md
description: Convert arXiv papers to LLM-friendly Markdown with high-signal extraction and robust fallback.
---

# arXiv to Markdown Skill

This skill provides instructions and workflows for using `arxiv2md` to convert academic papers from arXiv into clean, high-signal Markdown optimized for LLM context windows.

## Installation

If `arxiv2md` is not already installed, follow these steps to build and install it from source:

1. **Clone the repository:**
   ```bash
   git clone https://github.com/alexferrari88/arxiv2md
   ```
2. **Build and install:**
   ```bash
   cd arxiv2md
   cargo install --path .
   ```
3. **Prerequisites:** Ensure `pandoc` is installed on your system for LaTeX source fallback support.

## Core Workflow

To convert an arXiv paper, use its ID (e.g., `2501.11120`) or full URL.

### Basic Extraction
```bash
arxiv2md <ID_OR_URL>
```

### High-Signal Configuration (Recommended for LLMs)
For the best results in LLM prompts, include metadata and remove noise:
```bash
arxiv2md <ID> --frontmatter --remove-inline-citations --output paper.md
```

### Section Filtering
Focus on specific parts of a paper (e.g., just Abstract and Conclusion):
```bash
arxiv2md <ID> --section-filter-mode include --sections "Abstract,Conclusion"
```

## Options Reference

| Flag | Purpose |
| :--- | :--- |
| `--frontmatter` | Adds YAML metadata (Title, Authors, Abstract) to the top. |
| `--remove-inline-citations` | Strips `[1]`, `[2, 3]` style citations for cleaner text. |
| `--keep-refs` | Retains the Bibliography/References section. |
| `--sections <CSV>` | Filter sections by title (case-insensitive). |
| `--include-tree` | Shows the document structure before the content. |
| `--refresh` | Forces a fresh fetch, bypassing the local cache. |

## Fallback Strategy
`arxiv2md` automatically attempts to resolve content in this order:
1. **Official arXiv HTML** (Highest quality)
2. **ar5iv HTML** (High quality)
3. **LaTeX Source via Pandoc** (Structural fidelity)
4. **PDF Text Extraction** (Best effort)

use std::path::PathBuf;

use clap::{Parser, ValueEnum};

use crate::model::{CliOptions, SectionFilterMode};

#[derive(Debug, Parser)]
#[command(
    name = "arxiv2md",
    version,
    about = "Convert arXiv papers into LLM-friendly Markdown"
)]
struct Args {
    #[arg(value_name = "ID_OR_URL")]
    input: String,
    #[arg(short, long, default_value = "-")]
    output: String,
    #[arg(long)]
    frontmatter: bool,
    #[arg(long)]
    keep_refs: bool,
    #[arg(long)]
    keep_toc: bool,
    #[arg(long)]
    remove_inline_citations: bool,
    #[arg(long)]
    include_tree: bool,
    #[arg(long)]
    refresh: bool,
    #[arg(long, value_name = "CSV")]
    sections: Option<String>,
    #[arg(long, value_name = "TITLE")]
    section: Vec<String>,
    #[arg(long, value_enum, default_value = "exclude")]
    section_filter_mode: SectionFilterModeArg,
    #[arg(long, value_name = "PATH")]
    cache_dir: Option<PathBuf>,
    #[arg(long, default_value = "pandoc", value_name = "PATH")]
    pandoc_path: String,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SectionFilterModeArg {
    Include,
    Exclude,
}

pub fn parse() -> CliOptions {
    let args = Args::parse();
    let mut sections = Vec::new();
    if let Some(csv) = args.sections {
        sections.extend(
            csv.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToOwned::to_owned),
        );
    }
    sections.extend(
        args.section
            .into_iter()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty()),
    );

    CliOptions {
        input: args.input,
        output: args.output,
        frontmatter: args.frontmatter,
        keep_refs: args.keep_refs,
        keep_toc: args.keep_toc,
        remove_inline_citations: args.remove_inline_citations,
        include_tree: args.include_tree,
        refresh: args.refresh,
        sections,
        section_filter_mode: match args.section_filter_mode {
            SectionFilterModeArg::Include => SectionFilterMode::Include,
            SectionFilterModeArg::Exclude => SectionFilterMode::Exclude,
        },
        cache_dir: args.cache_dir,
        pandoc_path: args.pandoc_path,
    }
}

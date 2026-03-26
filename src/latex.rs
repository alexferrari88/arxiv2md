use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;

use flate2::read::GzDecoder;
use walkdir::WalkDir;

use crate::error::{Arxiv2MdError, Result};

pub fn convert_source_archive(bytes: &[u8], pandoc_path: &str) -> Result<String> {
    let workdir = tempfile::tempdir().map_err(|error| Arxiv2MdError::Latex(error.to_string()))?;
    unpack_archive(bytes, workdir.path())?;
    let main_tex = select_main_tex(workdir.path())?;

    let output = Command::new(pandoc_path)
        .current_dir(main_tex.parent().unwrap_or(workdir.path()))
        .arg("-f")
        .arg("latex")
        .arg("-t")
        .arg("gfm")
        .arg(main_tex.file_name().unwrap_or_default())
        .output()
        .map_err(|error| Arxiv2MdError::Latex(error.to_string()))?;

    if !output.status.success() {
        return Err(Arxiv2MdError::Latex(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn unpack_archive(bytes: &[u8], destination: &Path) -> Result<()> {
    let mut tar = tar::Archive::new(Cursor::new(bytes));
    if tar.unpack(destination).is_ok() {
        return Ok(());
    }

    let decoder = GzDecoder::new(Cursor::new(bytes));
    let mut gzip = tar::Archive::new(decoder);
    gzip.unpack(destination)
        .map_err(|error| Arxiv2MdError::Latex(error.to_string()))
}

fn select_main_tex(root: &Path) -> Result<PathBuf> {
    let mut tex_files = WalkDir::new(root)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .is_some_and(|extension| extension == "tex")
        })
        .filter_map(|entry| {
            fs::read_to_string(entry.path())
                .ok()
                .map(|contents| (entry.path().to_path_buf(), contents))
        })
        .collect::<Vec<_>>();

    if tex_files.is_empty() {
        return Err(Arxiv2MdError::Latex("no .tex files found".into()));
    }

    tex_files.sort_by_key(|(path, contents)| {
        let has_documentclass = contents.contains("\\documentclass");
        let name = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase();
        let supplementary = ["supp", "supplement", "appendix", "si", "supplementary"]
            .iter()
            .any(|marker| name.contains(marker));
        let preferred_name = ["main", "paper", "article", "ms"]
            .iter()
            .any(|marker| name == *marker);
        (
            !has_documentclass,
            supplementary,
            !preferred_name,
            std::cmp::Reverse(contents.len()),
        )
    });

    Ok(tex_files.remove(0).0)
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use tar::{Builder, Header};

    use super::convert_source_archive;

    #[test]
    fn converts_simple_latex_archive_with_pandoc() {
        let latex = r#"\documentclass{article}
\begin{document}
\section{Intro}
Hello from pandoc.
\end{document}
"#;

        let mut bytes = Vec::new();
        {
            let mut builder = Builder::new(&mut bytes);
            let mut header = Header::new_gnu();
            header.set_path("main.tex").expect("path");
            header.set_size(latex.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append(&header, Cursor::new(latex.as_bytes()))
                .expect("append");
            builder.finish().expect("finish");
        }

        let markdown =
            convert_source_archive(&bytes, "pandoc").expect("pandoc conversion succeeds");
        assert!(markdown.contains("## Intro") || markdown.contains("# Intro"));
        assert!(markdown.contains("Hello from pandoc."));
    }
}

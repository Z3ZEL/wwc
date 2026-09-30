//! Trunk `post_build` hook (frontend/Trunk.toml). Reads `assets/seo.json` and
//! `assets/theme.json` from `TRUNK_SOURCE_DIR`, then fills `index.html` and writes
//! robots.txt, sitemap.xml and the SEO images into `TRUNK_STAGING_DIR`. It also warns
//! about placeholder values left in `assets/documents/documents.json` (the legal pages).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::{env, fs};

use seo_gen::{Seo, ThemeColors, document_placeholders, inject};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("seo-gen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let source = dir("TRUNK_SOURCE_DIR")?;
    let staging = dir("TRUNK_STAGING_DIR")?;
    let assets = source.join("assets");

    let seo = Seo::parse(&read(&assets.join("seo.json"))?)?;
    for warning in seo.validate()? {
        eprintln!("seo-gen: warning: {warning}");
    }
    let theme = ThemeColors::parse(&read(&assets.join("theme.json"))?)?;
    let placeholders = document_placeholders(&read(&assets.join("documents").join("documents.json"))?)?;
    if !placeholders.is_empty() {
        eprintln!(
            "seo-gen: warning: the legal pages still show placeholders: set {} in assets/documents/documents.json \
             before deploying",
            placeholders.join(", ")
        );
    }

    let index = staging.join("index.html");
    write(&index, &inject(&read(&index)?, &seo, &theme)?)?;
    write(&staging.join("robots.txt"), &seo.render_robots())?;
    write(&staging.join("sitemap.xml"), &seo.render_sitemap())?;
    for file in [&seo.favicon, &seo.open_graph.image] {
        let from = assets.join("seo").join(file);
        fs::copy(&from, staging.join(file)).map_err(|e| format!("copy {}: {e}", from.display()))?;
    }
    Ok(())
}

fn dir(var: &str) -> Result<PathBuf, String> {
    env::var_os(var).map(PathBuf::from).ok_or_else(|| format!("{var} is not set (run through Trunk)"))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))
}

fn write(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("write {}: {e}", path.display()))
}

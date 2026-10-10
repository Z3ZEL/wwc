//! Trunk `post_build` hook (frontend/Trunk.toml). Writes `releases.json` into
//! `TRUNK_STAGING_DIR`: the latest GitHub Releases of the repository named in
//! `assets/updates.json`, for the Updates tab of the welcome card (ARCHITECTURE §5.6, ADR 0020).
//!
//! Release builds call the GitHub API, with `GITHUB_TOKEN` as a bearer token when it is set.
//! Dev builds use the recorded fixture (`WWC_CHANGELOG=fetch|fixture|off` overrides).
//! A failed call only warns and writes an empty list: release notes never block a deploy.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;
use std::{env, fs};

use changelog_gen::{FIXTURE, OUTPUT, Settings, Source, render, transform};

const TIMEOUT: Duration = Duration::from_secs(20);

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("changelog-gen: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let source = dir("TRUNK_SOURCE_DIR")?;
    let staging = dir("TRUNK_STAGING_DIR")?;
    let settings = Settings::parse(&read(&source.join("assets").join("updates.json"))?)?;
    let from = Source::choose(env::var("WWC_CHANGELOG").ok().as_deref(), env::var("TRUNK_PROFILE").ok().as_deref())?;

    let releases = match from {
        Source::Off => vec![],
        Source::Fixture => transform(FIXTURE, &settings)?,
        Source::Fetch => match fetch(&settings).and_then(|json| transform(&json, &settings)) {
            Ok(releases) => {
                eprintln!("changelog-gen: {} releases from {}", releases.len(), settings.repo);
                releases
            }
            Err(e) => {
                eprintln!("changelog-gen: warning: {e}. The Updates tab will be empty.");
                vec![]
            }
        },
    };
    write(&staging.join(OUTPUT), &render(&releases)?)
}

fn fetch(settings: &Settings) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).build().into();
    let mut request = agent
        .get(settings.api_url())
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "wwc-changelog-gen");
    // Optional: unauthenticated calls are limited to 60 an hour per IP, shared by the build host.
    if let Some(token) = env::var("GITHUB_TOKEN").ok().map(|t| t.trim().to_owned()).filter(|t| !t.is_empty()) {
        request = request.header("Authorization", format!("Bearer {token}"));
    }
    let mut response = request.call().map_err(|e| match e {
        ureq::Error::StatusCode(code @ (403 | 429)) => {
            format!("GitHub answered HTTP {code} (rate limit? set GITHUB_TOKEN)")
        }
        ureq::Error::StatusCode(code) => format!("GitHub answered HTTP {code}"),
        e => format!("could not reach GitHub: {e}"),
    })?;
    response.body_mut().read_to_string().map_err(|e| format!("could not read GitHub's answer: {e}"))
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

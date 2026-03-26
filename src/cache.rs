use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use directories::ProjectDirs;

use crate::error::{Arxiv2MdError, Result};
use crate::model::{ArxivId, CachePolicy};

const VERSIONLESS_TTL: Duration = Duration::from_secs(60 * 60 * 24);

pub fn build_policy(
    id: &ArxivId,
    override_dir: Option<PathBuf>,
    refresh: bool,
) -> Result<CachePolicy> {
    let dir = match override_dir {
        Some(path) => path,
        None => {
            let project = ProjectDirs::from("org", "alex", "arxiv2md").ok_or_else(|| {
                Arxiv2MdError::Cache("project dirs".into(), "could not resolve cache dir".into())
            })?;
            project.cache_dir().to_path_buf()
        }
    };

    Ok(CachePolicy {
        dir: dir.join(sanitize_key(&id.normalized)),
        ttl: if id.versioned_input || id.version.is_some() {
            None
        } else {
            Some(VERSIONLESS_TTL)
        },
        refresh,
    })
}

pub fn read_text(policy: &CachePolicy, name: &str) -> Result<Option<String>> {
    let path = policy.dir.join(name);
    if !is_fresh(&path, policy.ttl, policy.refresh)? {
        return Ok(None);
    }
    fs::read_to_string(&path)
        .map(Some)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))
}

pub fn write_text(policy: &CachePolicy, name: &str, body: &str) -> Result<()> {
    let path = policy.dir.join(name);
    ensure_dir(&policy.dir)?;
    fs::write(&path, body)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))
}

pub fn read_bytes(policy: &CachePolicy, name: &str) -> Result<Option<Vec<u8>>> {
    let path = policy.dir.join(name);
    if !is_fresh(&path, policy.ttl, policy.refresh)? {
        return Ok(None);
    }
    fs::read(&path)
        .map(Some)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))
}

pub fn write_bytes(policy: &CachePolicy, name: &str, body: &[u8]) -> Result<()> {
    let path = policy.dir.join(name);
    ensure_dir(&policy.dir)?;
    let mut file = fs::File::create(&path)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))?;
    file.write_all(body)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))
}

fn ensure_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))
}

fn is_fresh(path: &Path, ttl: Option<Duration>, refresh: bool) -> Result<bool> {
    if refresh || !path.exists() {
        return Ok(false);
    }
    let Some(ttl) = ttl else {
        return Ok(true);
    };
    let metadata = fs::metadata(path)
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))?;
    let modified = metadata
        .modified()
        .map_err(|error| Arxiv2MdError::Cache(path.display().to_string(), error.to_string()))?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or(Duration::ZERO);
    Ok(age <= ttl)
}

fn sanitize_key(value: &str) -> String {
    value.replace('/', "__")
}

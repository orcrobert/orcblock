//! Site icons for the blocklist: fetched once per domain, checked, cached
//! on disk and handed to the UI as data URLs.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use orcblock_core::Domain;
use sha2::{Digest as _, Sha256};

/// The largest icon accepted.
const MAX_BYTES: u64 = 256 * 1024;
/// How long a site without a usable icon is remembered before trying again.
const MISS_TTL: Duration = Duration::from_hours(24);
const TIMEOUT: Duration = Duration::from_secs(5);
/// Redirects followed per icon URL, and only within the site.
const MAX_REDIRECTS: usize = 3;

/// Fetches and caches favicons.
#[derive(Debug)]
pub(crate) struct Favicons {
    cache_dir: PathBuf,
    agent: ureq::Agent,
}

/// Why one icon URL gave no icon.
#[derive(Debug, PartialEq, Eq)]
enum Failure {
    /// The site answered, but not with a usable icon.
    Missing,
    /// The site could not be reached, for example because it is blocked
    /// right now. Not cached, so the icon comes back after the block.
    Unreachable,
}

impl From<ureq::Error> for Failure {
    fn from(err: ureq::Error) -> Self {
        match err {
            ureq::Error::StatusCode(_) | ureq::Error::BodyExceedsLimit(_) => Self::Missing,
            _ => Self::Unreachable,
        }
    }
}

impl Favicons {
    pub(crate) fn new(cache_dir: PathBuf) -> Self {
        // Redirects are followed by hand so they cannot leave the site or
        // drop to plain http; no proxy, so the blocklist stays on this Mac.
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(concat!("OrcBlock/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .max_redirects(0)
            .proxy(None)
            .build()
            .into();
        Self { cache_dir, agent }
    }

    /// The icon for `domain` as a data URL, or `None` when the site has no
    /// usable icon (the UI shows an empty square instead).
    pub(crate) fn get(&self, domain: &Domain) -> Option<String> {
        let key = cache_key(domain);
        let hit = self.cache_dir.join(format!("{key}.img"));
        let miss = self.cache_dir.join(format!("{key}.miss"));
        if let Ok(bytes) = fs::read(&hit) {
            return data_url(&bytes);
        }
        if is_fresh(&miss) {
            return None;
        }
        let found = self.fetch(domain);
        let written = match &found {
            Ok(bytes) => write_atomic(&self.cache_dir, &hit, bytes),
            Err(Failure::Missing) => write_atomic(&self.cache_dir, &miss, b""),
            Err(Failure::Unreachable) => Ok(()),
        };
        if let Err(err) = written {
            tracing::debug!(%err, "favicon cache not written");
        }
        found.ok().as_deref().and_then(data_url)
    }

    fn fetch(&self, domain: &Domain) -> Result<Vec<u8>, Failure> {
        let host = domain.as_ascii();
        let mut failure = Failure::Missing;
        for url in [
            format!("https://{host}/apple-touch-icon.png"),
            format!("https://{host}/favicon.ico"),
        ] {
            match self.download(domain, url) {
                Ok(bytes) if sniff(&bytes).is_some() => return Ok(bytes),
                Ok(_) | Err(Failure::Missing) => {}
                Err(Failure::Unreachable) => failure = Failure::Unreachable,
            }
        }
        Err(failure)
    }

    fn download(&self, domain: &Domain, mut url: String) -> Result<Vec<u8>, Failure> {
        for _ in 0..=MAX_REDIRECTS {
            let mut response = self.agent.get(&url).call()?;
            if !response.status().is_redirection() {
                return Ok(response
                    .body_mut()
                    .with_config()
                    .limit(MAX_BYTES)
                    .read_to_vec()?);
            }
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .ok_or(Failure::Missing)?;
            url = same_site_redirect(domain, location).ok_or(Failure::Missing)?;
        }
        Err(Failure::Missing)
    }
}

/// Where a redirect may go: an https URL on `domain` or one of its
/// subdomains (`example.com` to `www.example.com`), or a path on `domain`.
fn same_site_redirect(domain: &Domain, location: &str) -> Option<String> {
    let host = domain.as_ascii();
    if location.starts_with('/') && !location.starts_with("//") {
        return Some(format!("https://{host}{location}"));
    }
    let uri: ureq::http::Uri = location.parse().ok()?;
    let target = uri.host()?.to_ascii_lowercase();
    let on_site = target == host || target.ends_with(&format!(".{host}"));
    (uri.scheme_str() == Some("https") && on_site).then(|| location.to_owned())
}

/// The cache file stem for `domain`: a hash, so names never collide and
/// never depend on what a domain may contain.
fn cache_key(domain: &Domain) -> String {
    Sha256::digest(domain.as_ascii().as_bytes()).iter().fold(
        String::with_capacity(64),
        |mut hex, byte| {
            // Writing to a String cannot fail.
            let _ = write!(hex, "{byte:02x}");
            hex
        },
    )
}

/// Writes through a fresh temp file and a rename, so readers never see a
/// half-written icon and a planted symlink at `path` is replaced, not
/// followed.
fn write_atomic(dir: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    fs::create_dir_all(dir)?;
    let temp = dir.join(format!(".{}.tmp", std::process::id()));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)?;
    let written = file.write_all(bytes).and_then(|()| fs::rename(&temp, path));
    if written.is_err() {
        let _ = fs::remove_file(&temp);
    }
    written
}

fn is_fresh(path: &Path) -> bool {
    fs::metadata(path)
        .and_then(|meta| meta.modified())
        .and_then(|modified| {
            SystemTime::now()
                .duration_since(modified)
                .map_err(io::Error::other)
        })
        .is_ok_and(|age| age < MISS_TTL)
}

/// The image type of `bytes` by magic number. SVG is not accepted: it can
/// carry scripts, and raster icons are enough at 24 px.
pub(crate) fn sniff(bytes: &[u8]) -> Option<&'static str> {
    match bytes {
        [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, ..] => Some("image/png"),
        [0x00, 0x00, 0x01, 0x00, ..] => Some("image/x-icon"),
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        [b'G', b'I', b'F', b'8', ..] => Some("image/gif"),
        [
            b'R',
            b'I',
            b'F',
            b'F',
            _,
            _,
            _,
            _,
            b'W',
            b'E',
            b'B',
            b'P',
            ..,
        ] => Some("image/webp"),
        _ => None,
    }
}

fn data_url(bytes: &[u8]) -> Option<String> {
    sniff(bytes).map(|mime| format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

#[cfg(test)]
mod tests;

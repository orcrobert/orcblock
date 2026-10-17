//! Site icons for the blocklist: fetched once per domain, checked, cached
//! on disk and handed to the UI as data URLs.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use orcblock_core::Domain;

/// The largest icon accepted.
const MAX_BYTES: u64 = 256 * 1024;
/// How long a failed lookup is remembered before trying again.
const MISS_TTL: Duration = Duration::from_hours(24);
const TIMEOUT: Duration = Duration::from_secs(5);

/// Fetches and caches favicons.
#[derive(Debug)]
pub(crate) struct Favicons {
    cache_dir: PathBuf,
    agent: ureq::Agent,
}

impl Favicons {
    pub(crate) fn new(cache_dir: PathBuf) -> Self {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(concat!("OrcBlock/", env!("CARGO_PKG_VERSION")))
            .build()
            .into();
        Self { cache_dir, agent }
    }

    /// The icon for `domain` as a data URL, or `None` when the site has no
    /// usable icon (the UI shows a monogram instead).
    pub(crate) fn get(&self, domain: &Domain) -> Option<String> {
        let hit = self.cache_dir.join(domain.as_ascii());
        let miss = self.cache_dir.join(format!("{}.none", domain.as_ascii()));
        if let Ok(bytes) = fs::read(&hit) {
            return data_url(&bytes);
        }
        if is_fresh(&miss) {
            return None;
        }
        let found = self.fetch(domain);
        let written = fs::create_dir_all(&self.cache_dir).and_then(|()| match &found {
            Some(bytes) => fs::write(&hit, bytes),
            None => fs::write(&miss, b""),
        });
        if let Err(err) = written {
            tracing::debug!(%err, "favicon cache not written");
        }
        found.as_deref().and_then(data_url)
    }

    fn fetch(&self, domain: &Domain) -> Option<Vec<u8>> {
        let host = domain.as_ascii();
        [
            format!("https://{host}/apple-touch-icon.png"),
            format!("https://{host}/favicon.ico"),
        ]
        .iter()
        .find_map(|url| {
            self.download(url)
                .ok()
                .filter(|bytes| sniff(bytes).is_some())
        })
    }

    fn download(&self, url: &str) -> Result<Vec<u8>, ureq::Error> {
        self.agent
            .get(url)
            .call()?
            .body_mut()
            .with_config()
            .limit(MAX_BYTES)
            .read_to_vec()
    }
}

fn is_fresh(path: &std::path::Path) -> bool {
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

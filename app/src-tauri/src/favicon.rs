//! Site icons for the blocklist: fetched once per domain, checked, cached
//! on disk and handed to the UI as data URLs.

use std::fmt::Write as _;
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use orcblock_core::Domain;
use sha2::{Digest as _, Sha256};

/// The largest icon accepted, and how much of a home page is read for its
/// `<link rel="icon">` tags.
const MAX_BYTES: u64 = 256 * 1024;
/// Icons declared by a home page that are tried before giving up.
const MAX_LINKS: usize = 3;
/// How long a site without a usable icon is remembered before trying again.
const MISS_TTL: Duration = Duration::from_hours(24);
const TIMEOUT: Duration = Duration::from_secs(5);
/// Redirects followed per URL; each hop must pass `resolve`.
const MAX_REDIRECTS: usize = 3;
/// Well-known icon paths, tried before the home page.
const ICON_PATHS: [&str; 2] = ["/apple-touch-icon.png", "/favicon.ico"];

/// Fetches and caches favicons.
#[derive(Debug)]
pub(crate) struct Favicons {
    cache_dir: PathBuf,
    web: Web,
    /// Numbers temp files: rows fetch their icons in parallel.
    writes: AtomicU64,
}

/// Why a URL or a site gave no icon.
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
            ureq::Error::StatusCode(_) => Self::Missing,
            _ => Self::Unreachable,
        }
    }
}

impl Favicons {
    pub(crate) fn new(cache_dir: PathBuf) -> Self {
        // Redirects are followed by hand so each hop passes `resolve`; no
        // proxy, so the blocklist stays on this computer.
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(TIMEOUT))
            .user_agent(concat!("OrcBlock/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .max_redirects(0)
            .proxy(None)
            .build()
            .into();
        Self {
            cache_dir,
            web: Web(agent),
            writes: AtomicU64::new(0),
        }
    }

    /// The icon for `domain` as a data URL, or `None` when the site has no
    /// usable icon (the UI shows an empty square instead).
    pub(crate) fn get(&self, domain: &Domain) -> Option<String> {
        self.get_with(&self.web, domain)
    }

    fn get_with(&self, http: &impl Http, domain: &Domain) -> Option<String> {
        let key = cache_key(domain);
        let hit = self.cache_dir.join(format!("{key}.img"));
        let miss = self.cache_dir.join(format!("{key}.miss"));
        if let Ok(bytes) = fs::read(&hit) {
            return data_url(&bytes);
        }
        if is_fresh(&miss) {
            return None;
        }
        let found = fetch(http, domain);
        let temp = || {
            self.cache_dir.join(format!(
                ".{}.{}.tmp",
                std::process::id(),
                self.writes.fetch_add(1, Ordering::Relaxed)
            ))
        };
        let written = match &found {
            Ok(bytes) => write_atomic(&temp(), &hit, bytes),
            Err(Failure::Missing) => write_atomic(&temp(), &miss, b""),
            Err(Failure::Unreachable) => Ok(()),
        };
        if let Err(err) = written {
            tracing::debug!(%err, "favicon cache not written");
        }
        found.ok().as_deref().and_then(data_url)
    }
}

/// The fetcher's only network access, faked in tests.
trait Http {
    /// The body at `url` after redirects, with the URL it came from.
    fn get(&self, url: String, body: Body) -> Result<(String, Vec<u8>), Failure>;
}

#[derive(Debug)]
struct Web(ureq::Agent);

/// Tries the site, then `www.` in front of it: some machines resolve only
/// the bare name to nothing (a leftover hosts file entry, say).
fn fetch(http: &impl Http, domain: &Domain) -> Result<Vec<u8>, Failure> {
    let host = domain.as_ascii();
    match fetch_from(http, host) {
        Err(Failure::Unreachable) => fetch_from(http, &format!("www.{host}")),
        found => found,
    }
}

/// The well-known icon paths, then the icons the home page links to. The
/// site counts as unreachable when no request got an answer, or when the
/// page linked icons that could not be reached (a blocked or offline CDN).
fn fetch_from(http: &impl Http, host: &str) -> Result<Vec<u8>, Failure> {
    let tried = ICON_PATHS.map(|path| format!("https://{host}{path}"));
    let mut answered = false;
    for url in &tried {
        match icon(http, url.clone()) {
            Ok(bytes) => return Ok(bytes),
            Err(Failure::Missing) => answered = true,
            Err(Failure::Unreachable) => {}
        }
    }
    if !answered {
        return Err(Failure::Unreachable);
    }
    let (page, html) = http.get(format!("https://{host}/"), Body::Prefix)?;
    let mut failure = Failure::Missing;
    for url in icon_links(&String::from_utf8_lossy(&html))
        .iter()
        .filter_map(|href| resolve(&page, href))
        .filter(|url| !tried.contains(url))
        .take(MAX_LINKS)
    {
        match icon(http, url) {
            Ok(bytes) => return Ok(bytes),
            Err(Failure::Unreachable) => failure = Failure::Unreachable,
            Err(Failure::Missing) => {}
        }
    }
    Err(failure)
}

fn icon(http: &impl Http, url: String) -> Result<Vec<u8>, Failure> {
    let (_, bytes) = http.get(url, Body::Whole)?;
    if sniff(&bytes).is_some() {
        Ok(bytes)
    } else {
        Err(Failure::Missing)
    }
}

impl Http for Web {
    fn get(&self, mut url: String, body: Body) -> Result<(String, Vec<u8>), Failure> {
        for _ in 0..=MAX_REDIRECTS {
            let mut response = self.0.get(&url).call()?;
            if !response.status().is_redirection() {
                // ureq's own body limit counts compressed bytes, so a small
                // gzip response could inflate far past it; cap what comes
                // out of the decoder instead.
                let mut bytes = Vec::new();
                response
                    .body_mut()
                    .as_reader()
                    .take(MAX_BYTES + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|err| {
                        // A body that does not decompress is the site's
                        // answer; a cut-off one may come through next time.
                        if err.kind() == io::ErrorKind::InvalidData {
                            Failure::Missing
                        } else {
                            Failure::Unreachable
                        }
                    })?;
                return capped(bytes, body).map(|bytes| (url, bytes));
            }
            let location = response
                .headers()
                .get("location")
                .and_then(|value| value.to_str().ok())
                .ok_or(Failure::Missing)?;
            url = resolve(&url, location).ok_or(Failure::Missing)?;
        }
        Err(Failure::Missing)
    }
}

/// How much of a response to keep.
#[derive(Debug, Clone, Copy)]
enum Body {
    /// All of it, or a miss when it is over `MAX_BYTES`: an icon.
    Whole,
    /// The first `MAX_BYTES`: a home page, whose icon links sit in the head.
    Prefix,
}

/// `bytes`, read up to one byte past `MAX_BYTES`, cut down to what `body`
/// keeps.
fn capped(mut bytes: Vec<u8>, body: Body) -> Result<Vec<u8>, Failure> {
    let max = usize::try_from(MAX_BYTES).unwrap_or(usize::MAX);
    match body {
        Body::Whole if bytes.len() > max => Err(Failure::Missing),
        Body::Whole => Ok(bytes),
        Body::Prefix => {
            bytes.truncate(max);
            Ok(bytes)
        }
    }
}

/// `reference` (a redirect target or a link's `href`) made absolute against
/// `base`, if the fetcher may go there: https only, on the default port, to
/// a host name that is a valid domain ending in a letter-only top-level
/// label. IP literals in any spelling (`10.0.0.1`, `0xa.0x1`) and
/// `localhost` never pass. This filters names, not addresses: a public name
/// that resolves to a private address still passes, and TLS against the
/// bundled public roots is what keeps that from reaching anything.
fn resolve(base: &str, reference: &str) -> Option<String> {
    let reference = reference.trim();
    let base: ureq::http::Uri = base.parse().ok()?;
    let url = if reference.starts_with("//") {
        format!("https:{reference}")
    } else if reference.starts_with('/') {
        format!("https://{}{reference}", base.authority()?)
    } else if reference.starts_with(['?', '#']) {
        format!("https://{}{}{reference}", base.authority()?, base.path())
    } else if reference
        .split(['/', '?', '#'])
        .next()
        .is_some_and(|first| first.contains(':'))
    {
        reference.to_owned()
    } else {
        let dir = base.path().rsplit_once('/').map_or("", |(dir, _)| dir);
        format!("https://{}{dir}/{reference}", base.authority()?)
    };
    let uri: ureq::http::Uri = url.parse().ok()?;
    let host = uri.host()?;
    let public = !host.ends_with('.') && Domain::parse(host).is_ok_and(|domain| is_public(&domain));
    let default_port = uri.port_u16().is_none_or(|port| port == 443);
    (uri.scheme_str() == Some("https") && public && default_port).then_some(url)
}

/// False for `localhost` names and for numeric hosts that `Domain` lets
/// through but the resolver reads as IPv4 (`0x7f.0x1`): a real top-level
/// label is letters only, or punycode.
fn is_public(domain: &Domain) -> bool {
    let name = domain.as_ascii();
    let tld = name.rsplit('.').next().unwrap_or_default();
    let letters = tld.bytes().all(|b| b.is_ascii_lowercase()) || tld.starts_with("xn--");
    letters && tld != "localhost"
}

/// The `href`s of a page's `<link>` icons, best first: apple-touch icons
/// (large PNGs), then plain icons. SVG icons are skipped; see `sniff`.
fn icon_links(html: &str) -> Vec<String> {
    let lower = html.to_ascii_lowercase();
    let mut links: Vec<(u8, String)> = Vec::new();
    let mut rest = 0;
    while let Some(start) = lower[rest..].find("<link").map(|at| rest + at) {
        let end = lower[start..]
            .find('>')
            .map_or(lower.len(), |at| start + at);
        let attrs = attributes(&html[start + "<link".len()..end]);
        rest = end;
        let value = |name: &str| {
            attrs
                .iter()
                .find(|(key, _)| key.eq_ignore_ascii_case(name))
                .map(|(_, value)| *value)
        };
        let (Some(rel), Some(href)) = (value("rel"), value("href")) else {
            continue;
        };
        let rel = rel.to_ascii_lowercase();
        let rank = if rel
            .split_whitespace()
            .any(|token| token.starts_with("apple-touch-icon"))
        {
            0
        } else if rel.split_whitespace().any(|token| token == "icon") {
            1
        } else {
            continue;
        };
        let svg = value("type").is_some_and(|kind| kind.to_ascii_lowercase().contains("svg"))
            || href
                .split(['?', '#'])
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .ends_with(".svg");
        if !svg && !href.trim().is_empty() {
            links.push((rank, href.replace("&amp;", "&")));
        }
    }
    links.sort_by_key(|(rank, _)| *rank);
    links.into_iter().map(|(_, href)| href).collect()
}

/// The `name=value` pairs inside a tag, quoted or not.
fn attributes(tag: &str) -> Vec<(&str, &str)> {
    let mut pairs = Vec::new();
    let mut rest = tag;
    loop {
        rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == '/');
        let name_end = rest
            .find(|c: char| c.is_whitespace() || c == '=' || c == '/')
            .unwrap_or(rest.len());
        if name_end == 0 {
            return pairs;
        }
        let name = &rest[..name_end];
        rest = rest[name_end..].trim_start();
        let Some(after) = rest.strip_prefix('=') else {
            continue;
        };
        rest = after.trim_start();
        let (value, after) = if let Some(quote @ ('"' | '\'')) = rest.chars().next() {
            let inner = &rest[1..];
            let close = inner.find(quote).unwrap_or(inner.len());
            (&inner[..close], inner.get(close + 1..).unwrap_or_default())
        } else {
            let close = rest.find(char::is_whitespace).unwrap_or(rest.len());
            (&rest[..close], &rest[close..])
        };
        pairs.push((name, value));
        rest = after;
    }
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

/// Writes through `temp`, a fresh file next to `path`, and a rename, so
/// readers never see a half-written icon and a planted symlink at `path` is
/// replaced, not followed.
fn write_atomic(temp: &Path, path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(dir) = temp.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)?;
    let written = file.write_all(bytes).and_then(|()| fs::rename(temp, path));
    if written.is_err() {
        let _ = fs::remove_file(temp);
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

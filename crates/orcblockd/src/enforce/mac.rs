use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::net::{IpAddr, ToSocketAddrs};

use orcblock_core::Domain;
use tracing::{info, warn};

use super::runner::CommandRunner;
use super::{EnforceError, Enforcer, pf, resolver};
use crate::dns::CANARIES;
use crate::{Layout, fsutil};

const PFCTL: &str = "/sbin/pfctl";
const DSCACHEUTIL: &str = "/usr/bin/dscacheutil";
const KILLALL: &str = "/usr/bin/killall";

/// Resolves a host name to the addresses connections to it may be open to.
type Lookup = Box<dyn Fn(&str) -> Vec<IpAddr> + Send>;

/// Enforces blocks on macOS: resolver files that route blocked domains to
/// the sinkhole, a pf anchor against DNS over TLS and HTTPS, cache flushes,
/// and killing open connections to sites as their block starts.
pub struct MacEnforcer<R> {
    layout: Layout,
    runner: R,
    port: u16,
    lookup: Lookup,
    token: Option<String>,
    applied: BTreeSet<Domain>,
}

impl<R> fmt::Debug for MacEnforcer<R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MacEnforcer")
            .field("layout", &self.layout)
            .field("port", &self.port)
            .field("token", &self.token)
            .field("applied", &self.applied)
            .finish_non_exhaustive()
    }
}

impl<R: CommandRunner> MacEnforcer<R> {
    /// An enforcer for the system under `layout`, pointing resolver files
    /// at the sinkhole on `port`. Picks up the pf token a previous run left.
    pub fn new(layout: Layout, runner: R, port: u16) -> Self {
        let token = fs::read_to_string(layout.data_dir().join("pf-token"))
            .ok()
            .and_then(|text| pf::parse_token(&format!("Token : {}", text.trim())));
        Self {
            layout,
            runner,
            port,
            lookup: Box::new(system_lookup),
            token,
            applied: BTreeSet::new(),
        }
    }

    #[cfg(test)]
    fn with_lookup(mut self, lookup: impl Fn(&str) -> Vec<IpAddr> + Send + 'static) -> Self {
        self.lookup = Box::new(lookup);
        self
    }

    fn wanted(domains: &BTreeSet<Domain>) -> BTreeSet<String> {
        domains
            .iter()
            .map(|domain| domain.as_ascii().to_owned())
            .chain(CANARIES.iter().map(|&name| name.to_owned()))
            .collect()
    }

    fn backup_dir(&self) -> std::path::PathBuf {
        self.layout.data_dir().join("resolver-backup")
    }

    fn token_file(&self) -> std::path::PathBuf {
        self.layout.data_dir().join("pf-token")
    }

    fn pfctl(&self, args: &[&str]) -> Result<super::runner::Output, EnforceError> {
        self.runner.run(PFCTL, args, None)
    }

    fn load_anchor(&self) -> Result<(), EnforceError> {
        self.runner
            .run(PFCTL, &["-a", pf::ANCHOR, "-f", "-"], Some(&pf::rules()))
            .map(drop)
    }

    fn anchor_loaded(&self) -> Result<bool, EnforceError> {
        Ok(self
            .pfctl(&["-a", pf::ANCHOR, "-s", "rules"])?
            .stdout
            .contains("port = 853"))
    }

    fn pf_enabled(&self) -> Result<bool, EnforceError> {
        Ok(pf::is_enabled(&self.pfctl(&["-s", "info"])?.stdout))
    }

    /// Takes a reference on pf being enabled, once; the token releases it.
    fn enable_pf(&mut self) -> Result<(), EnforceError> {
        if self.token.is_some() {
            return Ok(());
        }
        let output = self.pfctl(&["-E"])?;
        let token = pf::parse_token(&output.stderr).ok_or_else(|| EnforceError::Program {
            program: PFCTL.to_owned(),
            reason: "pfctl -E printed no token".to_owned(),
        })?;
        let path = self.token_file();
        fs::create_dir_all(self.layout.data_dir())
            .and_then(|()| fsutil::write_atomic(&path, token.as_bytes(), 0o600))
            .map_err(|source| EnforceError::Io {
                action: "write",
                path,
                source,
            })?;
        self.token = Some(token);
        Ok(())
    }

    fn release_pf(&mut self) -> Result<(), EnforceError> {
        if let Some(token) = self.token.take() {
            self.pfctl(&["-X", &token])?;
        }
        let path = self.token_file();
        fsutil::remove_if_exists(&path).map_err(|source| EnforceError::Io {
            action: "remove",
            path,
            source,
        })
    }

    fn flush_caches(&self) -> Result<(), EnforceError> {
        self.runner.run(DSCACHEUTIL, &["-flushcache"], None)?;
        self.runner
            .run(KILLALL, &["-HUP", "mDNSResponder"], None)
            .map(drop)
    }

    /// Drops pf states to `ips` so open tabs lose their connections.
    fn kill_connections(&self, ips: &BTreeSet<IpAddr>) {
        for ip in ips {
            let any = if ip.is_ipv4() { "0.0.0.0/0" } else { "::/0" };
            let ip = ip.to_string();
            if let Err(err) = self.pfctl(&["-k", any, "-k", &ip]) {
                warn!(%err, "could not drop connections to {ip}");
            }
        }
    }
}

impl<R: CommandRunner> Enforcer for MacEnforcer<R> {
    fn apply(&mut self, domains: &BTreeSet<Domain>) -> Result<(), EnforceError> {
        // Resolve before the resolver files route these names to the sinkhole.
        let ips: BTreeSet<IpAddr> = domains
            .difference(&self.applied)
            .flat_map(|domain| {
                let name = domain.as_ascii();
                let mut ips = (self.lookup)(name);
                ips.extend((self.lookup)(&format!("www.{name}")));
                ips
            })
            .filter(|ip| !ip.is_unspecified() && !ip.is_loopback())
            .collect();

        let dir = self.layout.resolver_dir();
        let changed = resolver::sync(&dir, &self.backup_dir(), &Self::wanted(domains), self.port)?;
        self.load_anchor()?;
        self.enable_pf()?;
        if changed {
            self.flush_caches()?;
            info!(count = domains.len(), "resolver files updated");
        }
        self.kill_connections(&ips);
        self.applied.clone_from(domains);
        Ok(())
    }

    fn verify(&mut self, domains: &BTreeSet<Domain>) -> Result<bool, EnforceError> {
        let files_drifted = resolver::drifted(
            &self.layout.resolver_dir(),
            &Self::wanted(domains),
            self.port,
        )?;
        let pf_off = !self.pf_enabled()?;
        if pf_off {
            // Someone ran `pfctl -d`; the old token no longer holds pf on.
            self.token = None;
        }
        let drifted = files_drifted || pf_off || !self.anchor_loaded()?;
        if drifted {
            self.apply(domains)?;
        }
        Ok(drifted)
    }

    fn clear(&mut self) -> Result<(), EnforceError> {
        let files = resolver::clear(&self.layout.resolver_dir(), &self.backup_dir(), self.port);
        let anchor = self.pfctl(&["-a", pf::ANCHOR, "-F", "all"]).map(drop);
        let token = self.release_pf();
        let flush = self.flush_caches();
        self.applied.clear();
        files.map(drop).and(anchor).and(token).and(flush)
    }
}

fn system_lookup(name: &str) -> Vec<IpAddr> {
    (name, 443)
        .to_socket_addrs()
        .map(|addrs| addrs.map(|addr| addr.ip()).collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests;

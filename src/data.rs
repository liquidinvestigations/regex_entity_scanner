//! Loading of the vendored reference data the validators consult.
//!
//! Everything here is read once at startup and then shared immutably across scanning threads.
//! `RES_VENDORED_DIR` points at the vendored tree — the bind mount in development, a directory
//! baked into the release image otherwise.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// The IANA list and whatever else the validators need to distinguish a real match from a
/// plausible-looking one.
#[derive(Debug, Default)]
pub struct VendoredData {
    /// Top-level domains, uppercased exactly as IANA publishes them.
    tlds: HashSet<String>,
}

impl VendoredData {
    /// Reads the vendored tree at `RES_VENDORED_DIR`, defaulting to `./vendored`.
    pub fn load_from_env() -> Result<Self> {
        let root = std::env::var("RES_VENDORED_DIR").unwrap_or_else(|_| "vendored".to_string());
        Self::load(Path::new(&root))
    }

    pub fn load(root: &Path) -> Result<Self> {
        let tld_file: PathBuf = root.join("data/iana/tlds-alpha-by-domain.txt");
        let raw = std::fs::read_to_string(&tld_file)
            .with_context(|| format!("reading the vendored TLD list at {}", tld_file.display()))?;

        let tlds: HashSet<String> = raw
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_uppercase)
            .collect();

        anyhow::ensure!(
            tlds.len() > 1000,
            "the vendored TLD list holds only {} entries, which means it is truncated",
            tlds.len()
        );

        Ok(Self { tlds })
    }

    /// Whether `label` is a registered top-level domain. This one membership test removes the bulk
    /// of what an email pattern otherwise matches — file names, version strings, `foo@2x.png`.
    pub fn is_known_tld(&self, label: &str) -> bool {
        self.tlds.contains(&label.to_uppercase())
    }

    pub fn tld_count(&self) -> usize {
        self.tlds.len()
    }
}

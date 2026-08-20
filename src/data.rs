//! Loading of the vendored reference data the validators consult.
//!
//! Everything here is read once at startup and then shared immutably across scanning threads.
//! `RES_VENDORED_DIR` points at the vendored tree — the bind mount in development, a directory
//! baked into the release image otherwise.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

/// What the IBAN registry says about one country's account numbers.
///
/// ISO 7064 mod-97-10 accepts roughly one malformed candidate in ninety-seven on its own. The
/// length and the positional structure are what close that gap, and they are per country — which
/// is why this is vendored data rather than a constant.
#[derive(Debug, Deserialize)]
pub struct IbanCountry {
    /// The country as the registry names it, which is not always the CLDR spelling.
    pub country: String,
    /// Total length of the compact IBAN, country code and check digits included.
    pub length: usize,
    /// The BBAN structure as a run of `<length>!<class>` groups, `n` digits, `a` letters,
    /// `c` alphanumerics.
    pub structure: String,
    /// Byte range of the bank identifier within the compact IBAN, where the registry's structure
    /// determines one. Branch identifier positions are not in this source and are not guessed.
    #[serde(default)]
    pub bank: Option<[usize; 2]>,
}

/// The IANA list and whatever else the validators and the explainer need to distinguish a real
/// match from a plausible-looking one, and to say what it belongs to.
#[derive(Debug, Default)]
pub struct VendoredData {
    /// Top-level domains, uppercased exactly as IANA publishes them.
    tlds: HashSet<String>,
    /// ISO 3166-1 alpha-2 code to English territory name, from CLDR.
    territories: HashMap<String, String>,
    /// ISO 3166-1 alpha-2 code to the IBAN registry's entry for that country.
    iban_registry: HashMap<String, IbanCountry>,
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

        let territory_file: PathBuf = root.join("data/cldr/territories-en.json");
        let raw = std::fs::read_to_string(&territory_file).with_context(|| {
            format!(
                "reading the vendored territory names at {}",
                territory_file.display()
            )
        })?;
        let territories: HashMap<String, String> =
            serde_json::from_str(&raw).context("parsing the vendored territory names")?;

        let iban_file: PathBuf = root.join("data/iban/registry.json");
        let raw = std::fs::read_to_string(&iban_file).with_context(|| {
            format!(
                "reading the vendored IBAN registry at {}",
                iban_file.display()
            )
        })?;
        let iban_registry: HashMap<String, IbanCountry> =
            serde_json::from_str(&raw).context("parsing the vendored IBAN registry")?;

        anyhow::ensure!(
            iban_registry.len() > 70,
            "the vendored IBAN registry holds only {} countries, which means it is truncated",
            iban_registry.len()
        );

        Ok(Self {
            tlds,
            territories,
            iban_registry,
        })
    }

    /// Whether `label` is a registered top-level domain. This one membership test removes the bulk
    /// of what an email pattern otherwise matches — file names, version strings, `foo@2x.png`.
    pub fn is_known_tld(&self, label: &str) -> bool {
        self.tlds.contains(&label.to_uppercase())
    }

    pub fn tld_count(&self) -> usize {
        self.tlds.len()
    }

    /// The English name of an ISO 3166-1 alpha-2 territory. A card that can say "United Kingdom"
    /// should not say "GB".
    pub fn territory_name(&self, alpha2: &str) -> Option<&str> {
        self.territories
            .get(&alpha2.to_uppercase())
            .map(String::as_str)
    }

    /// Whether `alpha2` names an actual country rather than one of CLDR's groupings or
    /// placeholders. Identifier schemes that encode a country encode an ISO 3166-1 country, so
    /// `EU`, `ZZ` and their kind have to be excluded or every eight-letter word ending in `ZZPP`
    /// becomes a bank code.
    pub fn is_country_code(&self, alpha2: &str) -> bool {
        /// CLDR publishes these alongside the countries: political and economic groupings, the
        /// unknown-region placeholder, and the two pseudo-locales used for translation testing.
        const NOT_COUNTRIES: [&str; 7] = ["EU", "EZ", "QO", "UN", "XA", "XB", "ZZ"];

        let alpha2 = alpha2.to_uppercase();
        !NOT_COUNTRIES.contains(&alpha2.as_str()) && self.territories.contains_key(&alpha2)
    }

    /// The IBAN registry entry for a country code, or `None` for a country that issues no IBANs —
    /// which is itself a rejection, and a decisive one.
    pub fn iban_country(&self, alpha2: &str) -> Option<&IbanCountry> {
        self.iban_registry.get(&alpha2.to_uppercase())
    }
}

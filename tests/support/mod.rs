//! Shared setup for the integration tests.

use std::path::PathBuf;
use std::sync::Arc;

use regex_entity_scanner::data::VendoredData;
use regex_entity_scanner::scan::Scanner;

/// The vendored tree next to the manifest, so tests do not depend on the environment the way the
/// binary does.
pub fn vendored_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendored")
}

pub fn scanner() -> Arc<Scanner> {
    let data = VendoredData::load(&vendored_dir()).expect(
        "the vendored data is missing; run `vendored/fetch-all.sh` inside the dev container",
    );
    Arc::new(Scanner::new(data).expect("compiling the rule set"))
}

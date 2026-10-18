//! Source-bound constructor input for native Sun4EE0A0.
//!
//! The three initialized-image words were observed through actual source and
//! Session startup with calibrated write watches. The Sun constructor was not
//! reached by that capture; sun_state owns its separate native constructor port.
use crate::{
    catalog::Catalog,
    combat_source_receipt::{self as receipt, VerifiedStartupReceipt},
    combat_value_source_context::{self, SourceFpContext},
    sun_state,
};
use aa_rules::sources::sha256;
use serde::Deserialize;

pub const REFERENCE_SHA256: &str =
    "615e66797737ef5a8503625a4047119751659b6460e39802fac966b7218e8f33";
const REFERENCE: &[u8] = include_bytes!(
    "../../../research/skirmish-20261016-sun-startup-capture/sun-startup-reference.json"
);

#[derive(Deserialize)]
struct Reference {
    schema: u32,
    sun_address: u32,
    words: [u32; 3],
    source_identities_sha256: String,
    baseline_source_capture_sha256: String,
    runtime_pins: std::collections::BTreeMap<String, String>,
    watched_addresses: [u32; 3],
    armed_threads: usize,
    native_writes_before_cutoff: usize,
    post_cutoff_calibration_probes: usize,
    completed_source_return: bool,
    completed_session_constructor: bool,
    observed_sun_constructor: bool,
    cleanup_succeeded: bool,
}

fn checked_reference() -> Result<Reference, String> {
    if sha256(REFERENCE) != REFERENCE_SHA256 {
        return Err("sealed Sun startup reference digest differs".into());
    }
    let value: Reference = serde_json::from_slice(REFERENCE)
        .map_err(|e| format!("sealed Sun startup reference is invalid: {e}"))?;
    if value.schema != 1
        || value.sun_address != 0xb2814c
        || value.source_identities_sha256 != receipt::SOURCES_SHA256
        || value.baseline_source_capture_sha256 != receipt::CAPTURE_SHA256
        || value.watched_addresses != [0xb2814c, 0xb28150, 0xb28154]
        || value.armed_threads == 0
        || value.native_writes_before_cutoff != 0
        || value.post_cutoff_calibration_probes != 3
        || !value.completed_source_return
        || !value.completed_session_constructor
        || value.observed_sun_constructor
        || !value.cleanup_succeeded
    {
        return Err("Sun startup reference is outside its observed target or scope".into());
    }
    for (name, expected) in [
        ("AA.exe", receipt::EXE_SHA256),
        ("D3D9.dll", receipt::RENDERER_SHA256),
        ("dgVoodoo.conf", receipt::RENDERER_CONFIG_SHA256),
    ] {
        if value.runtime_pins.get(name).map(String::as_str) != Some(expected) {
            return Err(format!("Sun startup reference {name} differs"));
        }
    }
    Ok(value)
}

/// No raw-word, Clone, Default or Deserialize constructor. This input is consumed
/// once to create the retained Sun owner; existing saved Load then changes that
/// same state. It does not attest arbitrary later native global immutability.
#[derive(Debug)]
pub struct ConstructorInput {
    words: [u32; 3],
    source_context: SourceFpContext,
}

impl ConstructorInput {
    pub fn from_verified_startup_reference(
        catalog: &Catalog<'_>,
        startup: &VerifiedStartupReceipt,
    ) -> Result<Self, String> {
        let proof = startup
            .context()
            .reference_startup()
            .ok_or("Sun startup requires the sealed source reference")?;
        if proof.native_exe_sha256 != receipt::EXE_SHA256
            || proof.source_identities_sha256 != receipt::SOURCES_SHA256
            || proof.capture_sha256 != receipt::CAPTURE_SHA256
        {
            return Err("Sun startup source reference target differs".into());
        }
        validate(catalog, startup.context())?;
        Ok(Self {
            words: checked_reference()?.words,
            source_context: startup.context().clone(),
        })
    }

    /// Recheck the destination at the actual native constructor boundary so an
    /// earlier input cannot bless a different or subsequently edited Catalog.
    pub fn create_sun(self, catalog: &Catalog<'_>) -> Result<sun_state::State, String> {
        validate(catalog, &self.source_context)?;
        Ok(sun_state::State::new(self.words))
    }
}

fn validate(catalog: &Catalog<'_>, context: &SourceFpContext) -> Result<(), String> {
    catalog.require_source_owner()?;
    if receipt::source_fingerprint(&catalog.actors.sources) != receipt::SOURCES_SHA256 {
        return Err("Sun constructor destination source identities differ".into());
    }
    let numeric = combat_value_source_context::bind_numeric(catalog, context)?;
    receipt::require_reference_numeric(&numeric)
}

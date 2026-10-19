//! Actual global owners read by the first Object::Init footprint/memory paths.
//!
//! The point owner is the fixed reference's PE zero-fill seed, carried through
//! actual native initialization with calibrated Windows data watches. It is
//! bound to the verified logical source/FP target before the known ported load
//! path can borrow it. This does not assert perpetual native immutability.

use crate::{
    catalog::Catalog,
    combat_source_receipt::{self as combat_receipt, VerifiedStartupReceipt},
    combat_value_source_context,
};
use aa_rules::sources::sha256;
use aa_spatial::collision::Shape;
use serde::{Deserialize, Serialize};

pub const POINT_REFERENCE_SHA256: &str =
    "d50ac9ddb3d22d77c933068ebedb0d77fe700e24ec3511141f618ad3b7d15700";
const POINT_REFERENCE: &[u8] = include_bytes!(
    "../../../research/skirmish-20261016-load-world-globals/point-startup-reference.json"
);

#[derive(Debug, Serialize)]
pub struct PointProvenance {
    pub reference_sha256: &'static str,
    pub native_exe_sha256: &'static str,
    pub source_identities_sha256: &'static str,
    pub scope: &'static str,
}

#[derive(Deserialize)]
struct PointReference {
    schema: u32,
    point_address: u32,
    words: [u32; 6],
    source_identities_sha256: String,
    runtime_pins: std::collections::BTreeMap<String, String>,
    baseline_source_capture_sha256: String,
    captures: [Capture; 2],
    all_six_words_watched_across_two_runs: bool,
}

#[derive(Deserialize)]
struct Capture {
    word_indices: [usize; 4],
    armed_threads: usize,
    native_writes_before_cutoff: usize,
    calibration_probes: usize,
    completed_source_return: bool,
    completed_session_constructor: bool,
    cleanup_succeeded: bool,
}

fn checked_point_reference() -> Result<PointReference, String> {
    if sha256(POINT_REFERENCE) != POINT_REFERENCE_SHA256 {
        return Err("sealed point startup reference digest differs".into());
    }
    let reference: PointReference = serde_json::from_slice(POINT_REFERENCE)
        .map_err(|e| format!("sealed point startup reference is invalid: {e}"))?;
    if reference.schema != 1
        || reference.point_address != 0xb3a8c4
        || reference.words[0] != 0
        || !reference.all_six_words_watched_across_two_runs
        || reference.baseline_source_capture_sha256 != combat_receipt::CAPTURE_SHA256
        || reference.source_identities_sha256 != combat_receipt::SOURCES_SHA256
    {
        return Err("point startup reference is outside the captured native target".into());
    }
    for (name, expected) in [
        ("AA.exe", combat_receipt::EXE_SHA256),
        ("D3D9.dll", combat_receipt::RENDERER_SHA256),
        ("dgVoodoo.conf", combat_receipt::RENDERER_CONFIG_SHA256),
    ] {
        if reference.runtime_pins.get(name).map(String::as_str) != Some(expected) {
            return Err(format!("point startup reference {name} differs"));
        }
    }
    let mut covered = [false; 6];
    for capture in &reference.captures {
        if capture.armed_threads == 0
            || capture.native_writes_before_cutoff != 0
            || capture.calibration_probes != 4
            || !capture.completed_source_return
            || !capture.completed_session_constructor
            || !capture.cleanup_succeeded
        {
            return Err("point startup trace or its calibration is incomplete".into());
        }
        for &index in &capture.word_indices {
            let Some(slot) = covered.get_mut(index) else {
                return Err("point startup trace has an invalid word index".into());
            };
            *slot = true;
        }
    }
    if covered.iter().any(|v| !v) {
        return Err("point startup traces omit a native shape word".into());
    }
    Ok(reference)
}

/// The six-word global shape at B3A8C4 returned by native4E3E90.
///
/// Private fields and no Default/Deserialize/raw-word constructor prevent a
/// caller from substituting an arbitrary shape for the native global owner.
/// The sealed native startup producer is the only construction path. Unported
/// services that could mutate this native global must be proved before they
/// can be added to the admitted load path; this is not an eternal constant.
#[derive(Debug)]
pub struct PointShape {
    shape: Shape,
    provenance: PointProvenance,
    source_context: combat_value_source_context::SourceFpContext,
}

impl PointShape {
    /// Bind the observed native startup seed to the same verified source/FP
    /// reference used by the source manager. Revalidating the destination
    /// catalog prevents transferring an old receipt to different inputs.
    pub fn from_verified_startup_reference(
        catalog: &Catalog<'_>,
        startup: &VerifiedStartupReceipt,
    ) -> Result<Self, String> {
        catalog.require_source_owner()?;
        if combat_receipt::catalog_source_fingerprint(catalog)
            != combat_receipt::SOURCES_SHA256
        {
            return Err("point startup destination source identities differ".into());
        }
        let proof = startup
            .context()
            .reference_startup()
            .ok_or_else(|| "point startup requires the sealed source reference".to_owned())?;
        if proof.native_exe_sha256 != combat_receipt::EXE_SHA256
            || proof.source_identities_sha256 != combat_receipt::SOURCES_SHA256
            || proof.capture_sha256 != combat_receipt::CAPTURE_SHA256
        {
            return Err("point startup source reference target differs".into());
        }
        let numeric = combat_value_source_context::bind_numeric(catalog, startup.context())?;
        combat_receipt::require_reference_numeric(&numeric)?;
        let reference = checked_point_reference()?;
        let words = reference.words;
        Ok(Self {
            shape: Shape {
                kind: words[0],
                radius_bits: words[1],
                bounds: [words[2], words[3], words[4], words[5]],
            },
            provenance: PointProvenance {
                reference_sha256: POINT_REFERENCE_SHA256,
                native_exe_sha256: combat_receipt::EXE_SHA256,
                source_identities_sha256: combat_receipt::SOURCES_SHA256,
                scope: "Observed initial PE zero-fill seed through native source completion and Session construction; retained for the known ported load path, not native perpetual immutability or a first-map observation",
            },
            source_context: startup.context().clone(),
        })
    }

    /// Call at the reached shape-read boundary. A valid point owner cannot
    /// attest a different or subsequently edited destination source target.
    pub fn validate_catalog(&self, catalog: &Catalog<'_>) -> Result<(), String> {
        catalog.require_source_owner()?;
        if combat_receipt::catalog_source_fingerprint(catalog)
            != self.provenance.source_identities_sha256
        {
            return Err("point owner destination source identities differ".into());
        }
        let numeric = combat_value_source_context::bind_numeric(catalog, &self.source_context)?;
        combat_receipt::require_reference_numeric(&numeric)
    }

    pub fn shape(&self) -> &Shape {
        &self.shape
    }

    pub fn provenance(&self) -> &PointProvenance {
        &self.provenance
    }
}

/// Original76AC90 reads current KKC_Session+F8 through [B36550]. The timeline
/// KKC_Sim is the distinct global B36554; MapResource+A8 is also unrelated.
///
/// Implement this for the actual retained session owner. Reads must observe
/// that owner's current field, including changes made by77A750; a cached
/// phase copied from map metadata, constructor values or a previous call is
/// not an implementation of this contract. There is no default value.
pub trait SessionPhase {
    fn current_phase(&self) -> Result<u32, String>;

    /// Native54EF10 reads current [B36550]+E4 before the World+48 branch.
    /// This is separate from +F8. Hosts that only supply phase must fail when
    /// this additional source is reached, rather than inventing application0.
    fn application_state(&self) -> Result<u32, String> {
        Err("required native Session+E4 application-state owner is unbound".into())
    }

    /// Native76C7C0 after4F09C0, restricted to the positive global World branch
    /// of765E00. The caller must lend the SAME current managed-load World's
    /// sealed revision, after its76AA90 publication; a numeric revision or an
    /// old/destroyed World does not establish that precondition. Implementors
    /// traverse the actual Session BC roster and read raw Player+28, which is
    /// distinct from certifying a complete765E10/VWorld replacement. A reached
    /// matching player still requires that actual replacement service.
    fn clear_kingdom_references_in_created_world(
        &self,
        _removed: aa_actors::ActorId,
        _world: &crate::world_revision::ConstructedWorldRevision,
    ) -> Result<(), String> {
        Err("required native76C7C0 actual Session BC/player28 owner is unbound".into())
    }
}

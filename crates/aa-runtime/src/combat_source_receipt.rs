//! Verified, fixed Windows reference target for source combat arithmetic.
//!
//! This attests the captured v14/dgVoodoo reference build. It never says that
//! the current Rust client's thread was debugged or ran the native engine.
//! No native code is executed. The portable verifier uses embedded sealed
//! reference receipts, then checks current logical assets and computed results.
//! The external-file verifier additionally checks the reference build bytes.
use crate::{
    catalog::Catalog,
    combat_source_completion::Globals,
    combat_value_source_context::{self as context, CsvBinding, CsvScope, SourceFpContext},
};
use aa_rules::{
    resources::SourceIdentity,
    sources::{Origin, V14Sources, sha256},
};
use serde::Serialize;

pub type Result<T> = std::result::Result<T, String>;
pub const CAPTURE_SHA256: &str = "dcd61117e17730cd02ce62d27452b98867e3b0675f8e7a2a008f0d63f35f7df6";
pub const FILE_MANIFEST_SHA256: &str =
    "1a8a9591ed6c9993561809440f606a628bbbd104ce9fbc5f223556bb756de377";
pub const EXE_SHA256: &str = "3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1";
pub const RENDERER_SHA256: &str =
    "6a0ca214784be04b7c8b547105aa9d79acf4dc26c0b6f8702b437ddca54058b2";
pub const RENDERER_CONFIG_SHA256: &str =
    "241951ccb0361e76c5a7c7afe4f481122a7b3ff5d428e606913aae89631291f6";
pub const SOURCES_SHA256: &str = "aa96125b653b33c0706e240b9bedec545698659a6f59181e9017e4b2e601632d";
/// Ordered IDs and CV-reached component bits; the encoder covers exactly
/// 1028*3 cache words plus eight means; unused component bits are not certified.
pub const COMBAT_RESULT_SHA256: &str =
    "89c2baaa07597bd227b10e0912e1012a43ac316db5edbc391338b267f4375927";
/// Component bits actually tested by 4BBF90/4618B0. Detection bit22 is outside
/// this dependency slice; an independent full-mask correction is separate.
pub const CV_COMPONENT_BITS: u32 = 0x245e9;

/// Immutable provenance for one captured reference, not caller-written labels.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ReferenceProvenance {
    pub capture_sha256: &'static str,
    pub file_manifest_sha256: &'static str,
    pub native_exe_sha256: &'static str,
    pub renderer_sha256: &'static str,
    pub renderer_config_sha256: &'static str,
    pub source_identities_sha256: &'static str,
    pub computed_result_sha256: &'static str,
    pub scope: &'static str,
}
/// Bytes are hash-checked only. The API does not launch any supplied program.
pub struct TargetFiles<'a> {
    pub native_exe: &'a [u8],
    pub renderer_dll: &'a [u8],
    pub renderer_config: &'a [u8],
    pub capture_receipt: &'a [u8],
    pub file_manifest_receipt: &'a [u8],
}
/// Sealed proof: fields and construction are private to this verifier module.
pub(crate) struct CheckedTarget {
    provenance: ReferenceProvenance,
}
impl CheckedTarget {
    pub(crate) fn provenance(&self) -> ReferenceProvenance {
        self.provenance.clone()
    }
    pub(crate) fn fp(&self) -> (u16, u16, u32, bool) {
        (0x027f, 0x0120, 0x1fbd, true)
    }
}
/// The resulting context can only be obtained after build, receipts, source
/// identities, all typed settings and complete root UVars have been checked.
pub struct VerifiedStartupReceipt {
    context: SourceFpContext,
    csv: CsvBinding,
    globals: Globals,
}
impl VerifiedStartupReceipt {
    pub fn verify(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        files: TargetFiles<'_>,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        for (name, bytes, expected) in [
            ("reference AA.exe", files.native_exe, EXE_SHA256),
            ("reference D3D9.dll", files.renderer_dll, RENDERER_SHA256),
            (
                "reference dgVoodoo.conf",
                files.renderer_config,
                RENDERER_CONFIG_SHA256,
            ),
        ] {
            if sha256(bytes) != expected {
                return Err(format!(
                    "{name} does not match the verified captured target"
                ));
            }
        }
        Self::verify_reference(
            catalog,
            sources,
            files.capture_receipt,
            files.file_manifest_receipt,
        )
    }
    /// Uses the fixed, embedded captured-reference receipts. This verifies the
    /// algorithm target and current logical source inputs, never an active
    /// native process or the current Rust thread's FP controls. Publication
    /// still requires independently computing the complete result fingerprint.
    /// No native executable, renderer DLL or renderer config is needed at run time.
    pub fn verify_embedded_reference(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
    ) -> Result<Self> {
        Self::verify_reference(
            catalog,
            sources,
            include_bytes!("../../../research/development10/native-source-capture-03/capture.json"),
            include_bytes!(
                "../../../research/development10/native-source-capture-03/runtime-files.json"
            ),
        )
    }
    fn verify_reference(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        capture_receipt: &[u8],
        file_manifest_receipt: &[u8],
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        for (name, bytes, expected) in [
            ("sealed native capture", capture_receipt, CAPTURE_SHA256),
            (
                "sealed runtime file manifest",
                file_manifest_receipt,
                FILE_MANIFEST_SHA256,
            ),
        ] {
            if sha256(bytes) != expected {
                return Err(format!(
                    "{name} does not match the verified captured target"
                ));
            }
        }
        if source_fingerprint(&catalog.actors.sources) != SOURCES_SHA256 {
            return Err(
                "logical definition source identities differ from the captured reference binding"
                    .into(),
            );
        }
        let checked = CheckedTarget {
            provenance: ReferenceProvenance {
                capture_sha256: CAPTURE_SHA256,
                file_manifest_sha256: FILE_MANIFEST_SHA256,
                native_exe_sha256: EXE_SHA256,
                renderer_sha256: RENDERER_SHA256,
                renderer_config_sha256: RENDERER_CONFIG_SHA256,
                source_identities_sha256: SOURCES_SHA256,
                computed_result_sha256: COMBAT_RESULT_SHA256,
                scope: "Verified isolated fresh-profile Windows/v14/dgVoodoo reference capture03; not an observation of the current Rust client",
            },
        };
        let fp = SourceFpContext::from_checked_reference(&checked)?;
        let numeric = context::bind_numeric(catalog, &fp)?;
        require_reference_numeric(&numeric)?;
        let csv = context::bind_csv_from_sources(
            sources,
            CsvScope::FreshNoProfile {
                label: "verified captured reference: Personal5 redirected before source entry; root-created flags0 CSV and native restricted flags3 user-UVars policy",
            },
        )?;
        let expected = [
            (
                "UVars.tgi",
                "c2d601af619103b7b2493e2f5a41d96f78bfc4cf8178e51dcb6fff3406489e9d",
            ),
            (
                "UVars_AA.tgi",
                "2d1aef9aed0749acf24ed179275b1678bc697014d952caa2cf10519e10d7594b",
            ),
            (
                "UVars_Restored.tgi",
                "d957c883d9a3273fef371ca1f52671516b31657c1736e42438a0d4e8cd3c07e2",
            ),
        ];
        if csv.inputs().len() != expected.len()
            || csv
                .inputs()
                .iter()
                .zip(expected)
                .any(|(actual, (name, hash))| actual.path != name || actual.sha256 != hash)
        {
            return Err("reference root UVars source set differs".into());
        }
        csv.require_empty_debug_stream()?;
        if csv.assignments().len() != 2
            || csv.assignments().iter().any(|a| !a.value_utf16.is_empty())
        {
            return Err("reference root CSV assignment sequence differs".into());
        }
        // The sealed capture has all three guards0 at source_entry and stream
        // +4==0. Native856DF0 returns false for that actual null stream owner.
        Ok(Self {
            context: fp,
            csv: csv.with_checked_reference(&checked),
            globals: Globals {
                assertion_guard: 0,
                ranged_lookup_guard: 0,
                csv_lookup_guard: 0,
                debug_stream_is_closed: true,
            },
        })
    }
    pub fn context(&self) -> &SourceFpContext {
        &self.context
    }
    pub fn csv(&self) -> &CsvBinding {
        &self.csv
    }
    pub fn globals(&self) -> Globals {
        self.globals
    }
}
/// Revalidate destination Settings at installation as well as receipt creation.
/// A captured-reference context is transferable data, not authority to bless a
/// second Catalog whose typed inputs differ from the reference target.
pub(crate) fn require_reference_numeric(numeric: &context::NumericBindings<'_, '_>) -> Result<()> {
    let expected = [
        ("CombatDamageMinimum", 0x3f800000),
        ("CombatValueRangePower", 0x3e19999a),
        ("CombatValueAreaPower", 0x3f800000),
        ("CombatValueOffenseWeight", 0x3f000000),
        ("CombatValueDefenseWeight", 0x3f000000),
        ("CombatValueMovementWeight", 0x3e4ccccd),
        ("CombatValueMovementDefault", 0x3f800000),
        ("CombatValueMeanValuesDiscardPercent", 0x3e4ccccd),
        ("CombatValueRangedMinimum", 0x41400000),
        ("CombatValueDefenseWeightRanged", 0x3e800000),
    ];
    if numeric.values().len() != expected.len() {
        return Err("reference numeric setting set differs".into());
    }
    for (name, bits) in expected {
        let actual = numeric.value(name)?;
        if actual.native_kind != 2 || actual.bits != bits {
            return Err(format!("typed reference numeric setting {name} differs"));
        }
    }
    Ok(())
}
fn text(out: &mut Vec<u8>, value: &str) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value.as_bytes());
}
/// Stable native-loader source-order identity encoding; excludes physical
/// paths, includes logical spelling, content hash and resolver member origin.
pub fn source_fingerprint(sources: &[SourceIdentity]) -> String {
    // Startup checks this for every placed object against the same immutable
    // source list. Reuse the last digest only for an identical input.
    thread_local! {
        static LAST: std::cell::RefCell<Option<(Vec<SourceIdentity>, String)>> =
            const { std::cell::RefCell::new(None) };
    }
    if let Some(hit) = LAST.with(|last| {
        last.borrow()
            .as_ref()
            .filter(|(input, _)| input.as_slice() == sources)
            .map(|(_, digest)| digest.clone())
    }) {
        return hit;
    }
    let digest = compute_source_fingerprint(sources);
    LAST.with(|last| *last.borrow_mut() = Some((sources.to_vec(), digest.clone())));
    digest
}
fn compute_source_fingerprint(sources: &[SourceIdentity]) -> String {
    let mut out = b"AA source identities v1\0".to_vec();
    out.extend_from_slice(&(sources.len() as u64).to_le_bytes());
    for source in sources {
        text(&mut out, &source.path);
        text(&mut out, &source.sha256);
        match source.origin {
            Origin::OriginalArchive { member_index } => {
                out.push(0);
                out.extend_from_slice(&(member_index as u64).to_le_bytes())
            }
            Origin::V14Loose => out.push(1),
        }
    }
    sha256(&out)
}
/// Encoder shared with State's required final publication check. IDs and
/// values are supplied by its already computed canonical Bank owner.
pub(crate) fn result_fingerprint(
    means: [u32; 8],
    rows: impl IntoIterator<Item = (String, u32, [u32; 3])>,
) -> String {
    let rows = rows.into_iter().collect::<Vec<_>>();
    let mut out = b"AA native combat cache v1\0".to_vec();
    out.extend_from_slice(&(rows.len() as u64).to_le_bytes());
    for value in means {
        out.extend_from_slice(&value.to_le_bytes())
    }
    for (id, flags, values) in rows {
        text(&mut out, &id);
        out.extend_from_slice(&(flags & CV_COMPONENT_BITS).to_le_bytes());
        for value in values {
            out.extend_from_slice(&value.to_le_bytes())
        }
    }
    sha256(&out)
}

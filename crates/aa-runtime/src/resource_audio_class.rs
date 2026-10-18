//!Actual Sound descriptor/class metadata at59CB00 in the same ResourceManager.
//!Current fresh-source policy is explicit; no decoder/Task-loaded state is set.
//!The user requested removal of artificial RAM/VRAM caps. The same growable
//!manager retains class identity/current weights without native fixed-budget
//!eviction. No zero-budget startup observation is implied by that policy.
use crate::{catalog::Catalog, definitions::Bank, load_view_variables::NativeOverrides};
use aa_numeric::Precision;
use aa_rules::{
    native::index::native_enumerate,
    resources::{Provenance, SourceIdentity},
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
#[path = "resource_audio_class_core.rs"]
mod core;
type Result<T> = std::result::Result<T, String>;

///Once-retained source/precision policy for the actual fresh no-profile
///application window. This contains no cache class or chosen weight. Future
///profile/console/current-variable writers require their real shared owner.
pub struct FreshCacheVariableScope {
    universe: Vec<SourceIdentity>,
    precision: Precision,
    variables: Vec<SourceIdentity>,
}
fn variable_paths(sources: &V14Sources) -> Vec<String> {
    let paths = sources
        .catalog()
        .into_iter()
        .map(|p| p.logical_path)
        .collect::<Vec<_>>();
    ["AVars*.tgi", "UVars*.tgi"]
        .into_iter()
        .flat_map(|pattern| native_enumerate(pattern, &paths))
        .collect()
}
impl FreshCacheVariableScope {
    pub(crate) fn variable_files(&self)->&[SourceIdentity]{&self.variables}
    pub(crate) fn from_sources(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err("ResourceCache current profile/console variable owner is required".into());
        }
        let bank = catalog.definition_bank.borrow();
        let source = bank.combat_source()?;
        source.completed_means()?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err("ResourceCache fresh source Bank differs".into());
        }
        let precision = source.source_precision();
        let mut variables = Vec::new();
        for path in variable_paths(sources) {
            let file = sources.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("ResourceCache variable bytes/hash differ".into());
            }
            variables.push(SourceIdentity {
                path: file.logical_path,
                sha256: file.sha256,
                origin: file.origin,
            });
        }
        Ok(Self {
            universe: catalog.actors.sources.clone(),
            precision,
            variables,
        })
    }
    pub(crate) fn require(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        source.completed_means()?;
        if self.universe != catalog.actors.sources
            || source.source_identities() != self.universe.as_slice()
            || source.source_precision() != self.precision
        {
            return Err(
                "ResourceCache fresh variable scope belongs to another source Bank/context".into(),
            );
        }
        Ok(())
    }
}

///Canonical9D02F0 descriptor, selected by real9D0900 WAV/MP3 registration.
///Native registry pointer/ordinal is replaced by this private typed owner.
pub struct AudioClass {
    ram_weight: i32,
    vram_weight: i32,
    declarations: Vec<Provenance>,
    files: Vec<SourceIdentity>,
}
impl AudioClass {
    pub fn name(&self) -> &'static str {
        "KKC_AudioResource"
    }
    pub fn ram_weight(&self) -> i32 {
        self.ram_weight
    }
    pub fn vram_weight(&self) -> i32 {
        self.vram_weight
    }
    pub fn declarations(&self) -> &[Provenance] {
        &self.declarations
    }
    pub fn source_files(&self) -> &[SourceIdentity] {
        &self.files
    }
    pub(crate) fn from_current(
        catalog: &Catalog<'_>,
        bank: &Bank,
        sources: &mut V14Sources,
        scope: &FreshCacheVariableScope,
    ) -> Result<Self> {
        scope.require(catalog, bank)?;
        if variable_paths(sources)
            != scope
                .variables
                .iter()
                .map(|v| v.path.clone())
                .collect::<Vec<_>>()
        {
            return Err(
                "ResourceCache current variable enumeration differs from fresh scope".into(),
            );
        }
        let mut values = Vec::new();
        let mut files = Vec::new();
        for expected in &scope.variables {
            let file = sources.read(&expected.path).map_err(|e| e.to_string())?;
            let actual = SourceIdentity {
                path: file.logical_path.clone(),
                sha256: file.sha256.clone(),
                origin: file.origin.clone(),
            };
            if &actual != expected || aa_rules::sources::sha256(&file.bytes) != expected.sha256 {
                return Err("ResourceCache current variable source changed".into());
            }
            let doc =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|v| v.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    if !doc
                        .property_name(p)
                        .is_some_and(|v| v.eq_ignore_ascii_case("ResourceCacheAudioRAMWeight"))
                    {
                        continue;
                    }
                    if p.left.len() != 2
                        || !doc
                            .text(p.left[0].span)
                            .is_some_and(|v| v.eq_ignore_ascii_case("int"))
                    {
                        return Err(
                            "ResourceCacheAudioRAMWeight actual typed Int producer is required"
                                .into(),
                        );
                    }
                    let value = doc
                        .property_value(p)
                        .ok_or("ResourceCache Audio weight is unassigned")?
                        .trim()
                        .parse::<i32>()
                        .map_err(
                            |_| "ResourceCache Audio weight lies outside literal Int domain",
                        )?;
                    values.push((
                        value,
                        Provenance {
                            path: actual.path.clone(),
                            sha256: actual.sha256.clone(),
                            span: p.span,
                        },
                    ));
                }
            }
            files.push(actual);
        }
        let current =
            match values.as_slice() {
                [] => None,
                [(value, _)] => Some(*value),
                _ => return Err(
                    "ResourceCache duplicate/reloaded Audio weight needs actual assignment owner"
                        .into(),
                ),
            };
        let (ram_weight, vram_weight) = core::weights(current)?;
        Ok(Self {
            ram_weight,
            vram_weight,
            declarations: values.into_iter().map(|(_, p)| p).collect(),
            files,
        })
    }
}

#[cfg(test)]
#[path = "resource_audio_class_tests.rs"]
mod tests;

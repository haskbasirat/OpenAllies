//!Canonical AudioData source-list owner and actual46B1D0 pre-request.
//!This selected46C3A0 projection owns only34/40/4C and their Sound40/41
//!effects. Audio playback/scalars/subtitles are not certified by this owner.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    fx_graph::RegisteredReference,
    resource_depot::{Backend, Resources},
    resource_objects::Handle,
};
use aa_numeric::Precision;
use aa_rules::{
    actors::ResolvedNode,
    native::TypedValue,
    resources::{Provenance, SourceIdentity},
};
use std::collections::BTreeMap;
#[path = "audio_data_request_core.rs"]
mod core;
type Result<T> = std::result::Result<T, String>;
struct Entry {
    source: Vec<Provenance>,
    precision: Precision,
    universe: Vec<SourceIdentity>,
    lists: [Vec<Handle>; 3],
    complete: bool,
    failed: Option<String>,
}
impl Entry {
    ///46B400 allocates empty lists with initial capacities3,5,3. Capacity is
    ///storage only; host vectors grow without a native fixed pool ceiling.
    fn constructed(
        source: Vec<Provenance>,
        precision: Precision,
        universe: Vec<SourceIdentity>,
    ) -> Self {
        Self {
            source,
            precision,
            universe,
            lists: [
                Vec::with_capacity(3),
                Vec::with_capacity(5),
                Vec::with_capacity(3),
            ],
            complete: false,
            failed: None,
        }
    }
}
///One persistent canonical Audio directory under the application's existing
///Data dispatcher. No baseData20 counter or second ResourceManager exists.
pub struct State {
    entries: BTreeMap<String, Entry>,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AudioDataState")
            .field("allocations", &self.entries.len())
            .field(
                "prepared",
                &self.entries.values().filter(|v| v.complete).count(),
            )
            .finish()
    }
}
impl State {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
    ///Prepare at the source phase. A failed partial list is retained and
    ///cannot be mistaken for a complete Audio allocation or silently retried.
    pub fn prepare(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        cap: &RegisteredReference<'_>,
        resources: &mut Resources<'_>,
    ) -> Result<()> {
        let (node, pc, folder) = source(catalog, bank, cap)?;
        let reference = cap.reference();
        let key = reference.id.to_ascii_lowercase();
        if let Some(entry) = self.entries.get(&key) {
            return require(entry, catalog, pc, &node);
        }
        self.entries.insert(
            key.clone(),
            Entry::constructed(node.sources(), pc, catalog.actors.sources.clone()),
        );
        let entry = self
            .entries
            .get_mut(&key)
            .expect("same just-inserted Audio allocation");
        let result: Result<()> = (|| {
            //46BD40's actual constructor registers0 SOUND,1 MUSIC,2 SPEECH.
            let category = text(&node, "category", pc)?;
            let category = core::category(&category)
                .ok_or("Audio category lookup/tail outside registered SOUND/MUSIC/SPEECH domain")?;
            let ignore = node
                .field("ignore_rate", pc)
                .map_err(error)?
                .ok_or("Audio ignore_rate field absent")?
                .boolean()
                .map_err(error)?;
            core::load_lists(
                &mut Loading {
                    catalog,
                    bank,
                    node: &node,
                    pc,
                    folder: &folder,
                    resources,
                },
                &mut entry.lists,
                category == 1 || category == 2,
                ignore,
            )?;
            entry.complete = true;
            Ok(())
        })();
        if let Err(error) = &result {
            entry.failed = Some(error.clone());
        }
        result
    }
    ///Runtime only borrows the already prepared SAME Sound handles. A
    ///missing directory entry never creates a source resource late.
    pub fn request(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        cap: &RegisteredReference<'_>,
        delta: i32,
        mode: u32,
        resources: &mut Resources<'_>,
        backend: &mut impl Backend,
    ) -> Result<()> {
        let (node, pc, _) = source(catalog, bank, cap)?;
        let entry = self
            .entries
            .get(&cap.reference().id.to_ascii_lowercase())
            .ok_or("Audio lists were not prepared at source time")?;
        require(entry, catalog, pc, &node)?;
        core::request(
            &mut Request {
                entry,
                resources,
                backend,
            },
            delta,
            mode,
        )
    }
    pub fn prepared_sound_lists<'a>(
        &'a self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        cap: &RegisteredReference<'_>,
    ) -> Result<[&'a [Handle]; 3]> {
        let (node, pc, _) = source(catalog, bank, cap)?;
        let entry = self
            .entries
            .get(&cap.reference().id.to_ascii_lowercase())
            .ok_or("Audio lists were not prepared at source time")?;
        require(entry, catalog, pc, &node)?;
        Ok([&entry.lists[0], &entry.lists[1], &entry.lists[2]])
    }
}
impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}
struct Loading<'a, 'assets, 'node, 'r> {
    catalog: &'a Catalog<'assets>,
    bank: &'a Bank,
    node: &'a ResolvedNode<'node>,
    pc: Precision,
    folder: &'a str,
    resources: &'a mut Resources<'r>,
}
impl core::SourceWorld for Loading<'_, '_, '_, '_> {
    type Sound = Handle;
    fn text(&mut self, list: usize) -> Result<String> {
        text(self.node, ["leadin", "files", "fallout"][list], self.pc)
    }
    fn resource(&mut self, token: &str, streaming: bool) -> Result<Handle> {
        let path = crate::data_pre_request::path::relative(self.folder, token)?;
        self.resources.sound_59b4b0(
            self.catalog,
            self.bank,
            &path.encode_utf16().collect::<Vec<_>>(),
            streaming,
        )
    }
    fn standard_rate(&mut self, sound: &Handle) -> Result<()> {
        self.resources.sound_standard_rate_40(sound)
    }
}
struct Request<'a, 'r, B> {
    entry: &'a Entry,
    resources: &'a mut Resources<'r>,
    backend: &'a mut B,
}
impl<B: Backend> core::World for Request<'_, '_, B> {
    type Sound = Handle;
    fn count(&mut self, list: usize) -> Result<usize> {
        Ok(self.entry.lists[list].len())
    }
    fn sound(&mut self, list: usize, index: usize) -> Result<Handle> {
        self.entry.lists[list]
            .get(index)
            .cloned()
            .ok_or("Audio current index lies outside its authoritative list".into())
    }
    fn request(&mut self, sound: Handle, delta: i32, mode: u32) -> Result<()> {
        self.resources.request_10(&sound, delta, mode, self.backend)
    }
}
fn source<'a>(
    catalog: &'a Catalog<'_>,
    bank: &Bank,
    cap: &RegisteredReference<'_>,
) -> Result<(ResolvedNode<'a>, Precision, String)> {
    cap.require_audio_reference(catalog, bank)?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("Audio source Bank universe differs".into());
    }
    let pc = source.source_precision();
    let reference = cap.reference();
    if reference.ordinal.is_some() || reference.cutscene {
        return Err("Audio graph capability is outside actual mode0 registry scope".into());
    }
    let registry = catalog
        .actors
        .native_data_registry("Audio")
        .ok_or("actual Audio registry missing")?;
    if registry.mode != Some(0) {
        return Err("Audio registry mode differs from482090".into());
    }
    let entry = registry
        .entries
        .iter()
        .find(|v| v.ids.eq_ignore_ascii_case(&reference.id))
        .ok_or("registered Audio allocation missing")?;
    let node = catalog.actors.native_data_node(entry).map_err(error)?;
    if node.sources() != reference.source || text(&node, "IDS", pc)? != entry.ids {
        return Err("Audio canonical graph/source identity differs".into());
    }
    let path = entry.source.path.replace('\\', "/");
    let folder = path.rsplit_once('/').map_or("", |(d, _)| d).to_owned();
    Ok((node, pc, folder))
}
fn require(
    entry: &Entry,
    catalog: &Catalog<'_>,
    pc: Precision,
    node: &ResolvedNode<'_>,
) -> Result<()> {
    if entry.source != node.sources()
        || entry.precision != pc
        || entry.universe != catalog.actors.sources
    {
        return Err("Audio retained source/Bank PC differs".into());
    }
    if !entry.complete {
        return Err(entry
            .failed
            .clone()
            .unwrap_or("Audio source lists are unfinished".into()));
    }
    Ok(())
}
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn text(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<String> {
    match node.field(name, pc).map_err(error)? {
        Some(TypedValue::String { value, .. }) if value.is_ascii() && !value.contains('\0') => {
            Ok(value)
        }
        _ => Err(format!("Audio {name} requires actual plain ASCII String")),
    }
}
#[cfg(test)]
#[path = "audio_data_pre_request_tests.rs"]
mod tests;

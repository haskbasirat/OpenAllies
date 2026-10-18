//! Actual MapResource+88 Creator values and their separate source-resource link.
//! Source parsing is an after-Load value binding; validation is preflight, not
//! native allocator/SEH or the entire resource-manager/selection UI protocol.
use crate::{
    catalog::Catalog,
    definitions::Bank,
    load_world_rules::CreatorRules,
    map_resource_streams::{ArchiveWitness, Owner as Streams},
};
use aa_mapworld::{
    SourceIdentity,
    header::{MapHeader, WorldCreator, WorldParameters},
};
use std::sync::{Arc, Weak};

pub(crate) struct CreatorWitness(Weak<()>);
impl CreatorWitness {
    pub(crate) fn same(&self, other: &Self) -> bool {
        self.0.strong_count() > 0 && other.0.strong_count() > 0
            && self.0.ptr_eq(&other.0)
    }
    pub(crate) fn matches(&self, creator: &Creator) -> bool {
        self.0.ptr_eq(&Arc::downgrade(&creator.identity))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Base,
    Blank,
    Load,
    Random,
    Preset,
    Metagame,
}

#[path = "launch_creators.rs"]
pub mod launch_creators;

///49AD70 key-only MapHeader load retains both actual CString words. Unknown
///TextManager fallback remains an owned refusal, never a manufactured empty.
#[derive(Clone)]
pub(crate) struct HeaderPair {
    key: Vec<u16>,
    text: Result<Vec<u16>, String>,
}
impl HeaderPair {
    fn bind(catalog: &Catalog<'_>, key: &[u16]) -> Self {
        let text = if key.is_empty() {
            Ok(Vec::new())
        } else {
            catalog.with_current_text(|table| {
                table.lookup(key).map_err(err)?.ok_or_else(||
                    "49AD70 Header missing-key TextManager+5C fallback producer is unbound".into())
            })
        };
        Self { key: key.to_vec(), text }
    }
    pub(crate) fn pair(&self) -> Result<crate::creator_labels::LocalizedPairRef<'_>, String> {
        Ok(crate::creator_labels::LocalizedPairRef::new(&self.key,
            self.text.as_deref().map_err(Clone::clone)?))
    }
}
#[derive(Clone)]
struct Named {
    ids: String,
    path: Vec<usize>,
    source: aa_rules::resources::Provenance,
}

/// The selected source MapResource remains a distinct, shared resource object.
/// Its parsed source Creator does not alias a launch or target Creator's arrays.
pub struct SourceResource {
    archive: ArchiveWitness,
    ids: Vec<u16>,
    header_source: SourceIdentity,
    creator: Creator,
    header: MapHeader,
    header_rules: CreatorRules,
    header_labels: crate::creator_labels::State,
    header_modified68: Vec<u8>,
    header_closed30: Vec<u8>,
    header_name: HeaderPair,
    header_description: HeaderPair,
}
/// No public Clone/Default/raw-array constructor. Native clone operations copy
/// the mutable value arrays but retain the SAME immutable source resource+50.
pub struct Creator {
    identity: Arc<()>,
    kind: Kind,
    rules: CreatorRules,
    parameters: WorldParameters,
    parameter_modified68: Vec<u8>,
    parameter_closed30: Vec<u8>,
    extents: [u32; 2],
    labels: crate::creator_labels::State,
    biome: Option<Named>,
    weather: Option<Named>,
    rng_seed: u32,
    source_resource: Option<Arc<SourceResource>>,
    source: Option<SourceIdentity>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn c_string(units: &[u16]) -> Result<String, String> {
    let n = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    String::from_utf16(&units[..n]).map_err(err)
}
fn named(
    catalog: &Catalog<'_>,
    manager: &str,
    serialized: &[u16],
) -> Result<Option<Named>, String> {
    //513700/4F9280 test CString length before Find. A nonempty string whose
    //prefix is empty must still perform the lookup and fail if unregistered.
    if serialized.is_empty() {
        return Ok(None);
    }
    let name = c_string(serialized)?;
    let registry = catalog
        .actors
        .native_data_registry(manager)
        .ok_or_else(|| format!("Creator {manager} registry absent"))?;
    let found = registry
        .find(&name)
        .ok_or_else(|| format!("Creator unknown {manager} IDS {name}"))?;
    catalog.actors.native_data_node(found).map_err(err)?;
    Ok(Some(Named {
        ids: found.ids.clone(),
        path: found.path.clone(),
        source: found.source.clone(),
    }))
}
fn resource_zeros(catalog: &Catalog<'_>, bank: &Bank) -> Result<Vec<(Vec<u16>, u32)>, String> {
    //50F4C0 sizes by GameData+38;4CD050 uses that SAME+34 resource order.
    let registry = catalog
        .actors
        .native_resource_registry(bank.combat_source()?.source_precision())
        .map_err(err)?;
    Ok(registry
        .ids
        .iter()
        .map(|id| (id.encode_utf16().collect(), 0))
        .collect())
}
fn materialize_parameters(
    catalog: &Catalog<'_>,
    mut parameters: WorldParameters,
) -> Result<WorldParameters, String> {
    let bank = catalog
        .definition_bank
        .try_borrow()
        .map_err(|_| "source Bank already borrowed")?;
    let zero = resource_zeros(catalog, &bank)?;
    let registry = catalog
        .actors
        .native_data_registry("Resource")
        .ok_or("Creator Resource manager absent")?;
    for record in &mut parameters.kingdoms {
        //The live loader's50F4C0 temporary supplies fields omitted by older
        //file versions. These are real constructor fields, not absent defaults.
        if record.string_0c.is_none() {
            record.string_0c = Some(Vec::new())
        }
        let saved = std::mem::take(&mut record.resources);
        let mut current = zero.clone();
        for (ids, bits) in saved {
            let name = c_string(&ids)?;
            let found = registry.find(&name).ok_or_else(|| {
                format!("Creator Resource {name} requires native unknown-resource log path")
            })?;
            catalog.actors.native_data_node(found).map_err(err)?;
            let index = current
                .iter()
                .position(|(id, _)| {
                    c_string(id).is_ok_and(|id| id.eq_ignore_ascii_case(&found.ids))
                })
                .ok_or("Creator Resource Find is outside completed native resource order")?;
            current[index].1 = bits;
        }
        record.resources = current;
    }
    Ok(parameters)
}
impl SourceResource {
    /// `ids` is the actual selected asset/resource key supplied by the resource
    /// lookup caller. An archive hash or header display label is not that key.
    pub fn load(
        catalog: &Catalog<'_>,
        streams: &Streams,
        ids: &[u16],
    ) -> Result<Arc<Self>, String> {
        if ids.is_empty() || ids.contains(&0) {
            return Err("source MapResource requires its actual nonempty file/resource IDS".into());
        }
        String::from_utf16(ids).map_err(err)?;
        let map = streams.archive()?;
        let archive = streams.witness();
        archive.validate_map(map)?;
        let creator = Creator::load_appended(catalog, map)?;
        //The original verified RMP decoder admits the actual base class only;
        //unknown polymorphic Creator subclasses do not become base instances.
        let chunk = map.decode_chunk("MAP_HEADER", u64::MAX).map_err(err)?;
        let mut header = MapHeader::decode(&chunk).map_err(err)?;
        header.parameters = materialize_parameters(catalog, header.parameters)?;
        let source = header.source.clone();
        let header_rules = crate::load_world_rules::Owner::load_header_rules(catalog, &header)?;
        let header_labels =
            crate::creator_labels::State::from_parameters(catalog, &header.parameters)?;
        //50F4C0 initializes +68 before the key-only parameter Load. No file
        //version serializes this byte, so the freshly loaded records retain 0.
        let header_modified68 = vec![0; header.parameters.kingdoms.len()];
        //50F4C0 constructs+30=0;510580 file Load does not write that field.
        let header_closed30 = vec![0; header.parameters.kingdoms.len()];
        let header_name = HeaderPair::bind(catalog, &header.name_key);
        let header_description = HeaderPair::bind(catalog, &header.description_key);
        //5A2A40's post-read current header writes. They do not replace the
        //independent source Creator's parameters with Header+90 parameters.
        if !crate::map_resource_mode::world_rule_mode(header.rules.mode)?.saved_state && header.rules.mode != 3 {
            header.string_38 = ids.to_vec();
        }
        header.display_extents_bits = Some(creator.extents);
        Ok(Arc::new(Self {
            archive,
            ids: ids.to_vec(),
            header_source: source,
            creator: *creator,
            header,
            header_rules,
            header_labels,
            header_modified68,
            header_closed30,
            header_name,
            header_description,
        }))
    }
    pub fn ids(&self) -> &[u16] {
        &self.ids
    }
    pub fn source(&self) -> &SourceIdentity {
        &self.header_source
    }
    pub fn parsed_creator(&self) -> &Creator {
        &self.creator
    }
    pub fn header(&self) -> &MapHeader {
        &self.header
    }
    pub fn header_rules(&self) -> &CreatorRules {
        &self.header_rules
    }
    pub fn header_labels(&self) -> &crate::creator_labels::State {
        &self.header_labels
    }
    pub fn header_modified68(&self) -> &[u8] {
        &self.header_modified68
    }
    pub(crate) fn header_closed30(&self)->&[u8] {&self.header_closed30}
    pub fn header_name_pair(&self) -> Result<crate::creator_labels::LocalizedPairRef<'_>, String> {
        self.header_name.pair()
    }
    pub fn header_description_pair(&self) -> Result<crate::creator_labels::LocalizedPairRef<'_>, String> {
        self.header_description.pair()
    }
    pub(crate) fn archive_witness(&self) -> &ArchiveWitness {
        &self.archive
    }
}
impl Creator {
    ///5141F0/513EB0/5137C0 fresh LaunchState CreatorLoad constructor.
    /// Current UVars/SVars are supplied by their retained controlled-start
    /// owner. The constructor's+50 is genuinely null until scenario Find.
    pub fn from_load_constructor(
        catalog: &Catalog<'_>,
        variables: &crate::creator_variables::State,
    ) -> Result<Box<Self>, String> {
        Self::from_common_constructor(catalog, variables, Kind::Load)
    }

    ///513EB0 shared base initialization. Only the exact derived constructor
    ///child module can supply another kind; no existing allocation is retagged.
    fn from_common_constructor(
        catalog: &Catalog<'_>,
        variables: &crate::creator_variables::State,
        kind: Kind,
    ) -> Result<Box<Self>, String> {
        variables.validate(catalog)?;
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|_| "source Bank already borrowed")?;
        let rules = CreatorRules::from_native_constructor(catalog, &bank)?;
        let biome = named(
            catalog,
            "Biome",
            &variables.default_biome().encode_utf16().collect::<Vec<_>>(),
        )?
        .ok_or("Creator default Biome Find is null")?;
        let resources = resource_zeros(catalog, &bank)?;
        //510DC0 resets the actual Kingdom vector, creates two major records,
        //then current DefaultKingdomsIndependent independent records. Resource
        //array45F180 is allocated to the actual registered count and zeroed.
        let mut kingdoms = Vec::new();
        for independent in [0u8, 0]
            .into_iter()
            .chain((0..variables.independent_kingdoms().max(0)).map(|_| 1))
        {
            kingdoms.push(aa_mapworld::header::KingdomParameter {
                id: Vec::new(),
                byte_10: independent,
                nation: Vec::new(),
                faction: Vec::new(),
                color: Vec::new(),
                handicap: Vec::new(),
                display_key: Vec::new(),
                string_0c: Some(Vec::new()),
                string_2c: Vec::new(),
                byte_29: 1,
                byte_28: 1,
                string_14: Vec::new(),
                localized_entries: Vec::new(),
                definitions: Vec::new(),
                string_58: Vec::new(),
                properties: Vec::new(),
                byte_31: 0,
                byte_32: 1,
                resources: resources.clone(),
            });
        }
        let parameters = WorldParameters {
            version: 13,
            teams: Vec::new(),
            kingdoms,
        };
        let labels = crate::creator_labels::State::from_native_constructor(&parameters)?;
        let parameter_modified68 = vec![0; parameters.kingdoms.len()];
        let parameter_closed30 = vec![0; parameters.kingdoms.len()];
        Ok(Box::new(Self {
            identity: Arc::new(()),
            kind,
            rules,
            parameters,
            parameter_modified68,
            parameter_closed30,
            labels,
            extents: [
                aa_numeric::Binary::from_i32(variables.width())
                    .store_f32()
                    .map_err(err)?,
                aa_numeric::Binary::from_i32(variables.height())
                    .store_f32()
                    .map_err(err)?,
            ],
            biome: Some(biome),
            weather: None,
            rng_seed: 0,
            source_resource: None,
            source: None,
        }))
    }

    ///5144A0 copies only the actual GameSettings parameter/rule values. It
    ///does not replace this Creator's dimensions, Biome, weather, seed or+50.
    pub(crate) fn assign_game_settings(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        parameters: &WorldParameters,
        rules: &CreatorRules,
        labels: &crate::creator_labels::State,
        modified68: &[u8],
        closed30: &[u8],
    ) -> Result<(), String> {
        self.validate(catalog, bank)?;
        rules.validate_in_bank(catalog, bank)?;
        labels.validate(parameters)?;
        if modified68.len() != parameters.kingdoms.len() || closed30.len()!=parameters.kingdoms.len() {
            return Err("Creator parameter +68 storage count differs".into());
        }
        self.parameters = parameters.clone();
        self.parameter_modified68 = modified68.to_vec();
        self.parameter_closed30 = closed30.to_vec();
        self.labels = labels.copy_for_creator();
        self.rules = rules.copy_for_creator();
        Ok(())
    }

    /// The actual selected source Header+90 -> GameSettings ->5144A0 value
    /// projection, before later host/UI mutations. The source resource's
    /// private header/rules/companions are used together; callers cannot mix a
    /// forged parameter vector with a canonical rule certificate.
    pub fn assign_source_header_game_settings(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        resource: &Arc<SourceResource>,
    ) -> Result<(), String> {
        resource.creator.validate(catalog, bank)?;
        self.assign_game_settings(
            catalog,
            bank,
            &resource.header.parameters,
            &resource.header_rules,
            &resource.header_labels,
            &resource.header_modified68,
            &resource.header_closed30,
        )
    }

    ///76B500 first copied the current Session+68 GameSettings owner. This
    ///5144A0 call reads that same retained owner at its actual call boundary.
    pub fn assign_current_game_settings(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        settings: &crate::game_settings_value::Owner,
    ) -> Result<(), String> {
        settings.validate(catalog, bank)?;
        self.assign_game_settings(
            catalog,
            bank,
            settings.parameters(),
            settings.rules(),
            settings.labels(),
            settings.modified68(),
            settings.closed30(),
        )
    }

    ///776B20's actual scenario Find writer, independent of5144A0. Null is a
    ///valid constructor value; positive load dereferences require Some later.
    pub fn assign_source_resource(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        resource: Arc<SourceResource>,
    ) -> Result<(), String> {
        if self.kind != Kind::Load {
            return Err("scenario source assignment requires actual CreatorLoad".into());
        }
        self.validate(catalog, bank)?;
        resource.creator.validate(catalog, bank)?;
        resource.header_rules.validate_in_bank(catalog, bank)?;
        self.source_resource = Some(resource);
        Ok(())
    }
    ///776B20 stores the current launch seed into the SAME selected Creator+4C.
    ///The caller supplies the actual launch RNG result at that write boundary.
    pub fn assign_rng_seed_776b20(&mut self,catalog:&Catalog<'_>,bank:&Bank,seed:u32)->Result<(),String> {
        self.validate(catalog,bank)?;
        self.rng_seed=seed;
        Ok(())
    }
    ///5A2A40/5A01F0 bind the appended source object, independently of the
    ///header+90 parameters and the launch-state CreatorLoad allocation.
    pub(crate) fn load_appended(
        catalog: &Catalog<'_>,
        map: &aa_mapworld::MapArchive,
    ) -> Result<Box<Self>, String> {
        let chunk = map.decode_chunk("MAP_HEADER", u64::MAX).map_err(err)?;
        let header = MapHeader::decode(&chunk).map_err(err)?;
        let source = header.source.clone();
        let rules = crate::load_world_rules::Owner::load_embedded_creator(catalog, map)?;
        let WorldCreator {
            class_name: _,
            version: _,
            rules: _,
            parameters,
            world_extents_bits,
            biome,
            weather,
            rng_seed,
        } = header.creator;
        let parameters = materialize_parameters(catalog, parameters)?;
        let labels = crate::creator_labels::State::from_parameters(catalog, &parameters)?;
        let parameter_modified68 = vec![0; parameters.kingdoms.len()];
        let parameter_closed30 = vec![0; parameters.kingdoms.len()];
        Ok(Box::new(Self {
            identity: Arc::new(()),
            kind: Kind::Base,
            rules,
            parameters,
            parameter_modified68,
            parameter_closed30,
            labels,
            extents: world_extents_bits,
            biome: named(catalog, "Biome", &biome)?,
            weather: named(catalog, "Weather", &weather)?,
            rng_seed,
            source_resource: None,
            source: Some(source),
        }))
    }
    fn copy_value(&self) -> Self {
        Self {
            identity: Arc::new(()),
            kind: self.kind,
            rules: self.rules.copy_for_creator(),
            parameters: self.parameters.clone(),
            parameter_modified68: self.parameter_modified68.clone(),
            parameter_closed30: self.parameter_closed30.clone(),
            labels: self.labels.copy_for_creator(),
            extents: self.extents,
            biome: self.biome.clone(),
            weather: self.weather.clone(),
            rng_seed: self.rng_seed,
            source_resource: self.source_resource.as_ref().map(Arc::clone),
            source: self.source.clone(),
        }
    }
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<(), String> {
        self.rules.validate_in_bank(catalog, bank)?;
        self.labels.validate(&self.parameters)?;
        if self.parameter_modified68.len() != self.parameters.kingdoms.len() || self.parameter_closed30.len()!=self.parameters.kingdoms.len() {
            return Err("Creator retained parameter +68 storage count differs".into());
        }
        for (manager, current) in [("Biome", &self.biome), ("Weather", &self.weather)] {
            if let Some(value) = current {
                let registry = catalog
                    .actors
                    .native_data_registry(manager)
                    .ok_or("Creator named registry absent")?;
                if !registry.entries.iter().any(|v| {
                    v.ids == value.ids
                        && v.path == value.path
                        && v.source == value.source
                        && v.reloads.is_empty()
                }) {
                    return Err("Creator retained canonical named definition differs".into());
                }
            }
        }
        match (self.kind, &self.source_resource) {
            (Kind::Base | Kind::Blank | Kind::Random | Kind::Preset | Kind::Metagame, None)
            | (Kind::Load, _) => Ok(()),
            _ => Err("Creator subclass/source-resource storage differs".into()),
        }
    }
    ///514550/514790 retain class and independently copy mutable rules/params.
    pub(crate) fn clone_for_resource(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
    ) -> Result<Box<Self>, String> {
        self.validate(catalog, bank)?;
        if !matches!(self.kind, Kind::Base | Kind::Load) {
            return Err("Creator derived clone requires its actual native tail constructor".into());
        }
        Ok(Box::new(self.copy_value()))
    }
    pub fn kind(&self) -> Kind {
        self.kind
    }
    pub(crate) fn witness(&self) -> CreatorWitness {
        CreatorWitness(Arc::downgrade(&self.identity))
    }
    pub fn source(&self) -> Option<&SourceIdentity> {
        self.source.as_ref()
    }
    pub fn rules(&self) -> &CreatorRules {
        &self.rules
    }
    ///The real511AF0 mutable rule instance inside this Creator. The argument
    ///is the canonical definition's current IDS, not a new source declaration.
    pub fn assign_bool_rule_current(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        canonical_rule_ids: &str,
        value: u8,
    ) -> Result<bool, String> {
        self.validate(catalog, bank)?;
        Ok(self.rules.assign_bool(canonical_rule_ids, value))
    }
    pub fn parameters(&self) -> &WorldParameters {
        &self.parameters
    }
    pub fn parameter_modified(&self, index: usize) -> Result<bool, String> {
        self.parameter_modified68
            .get(index)
            .map(|value| *value != 0)
            .ok_or_else(|| "Creator parameter +68 index outside actual vector".into())
    }
    pub(crate) fn closed30(&self)->&[u8] {&self.parameter_closed30}
    ///777CD0/777610 update the same parameter and modified-byte allocations.
    pub(crate) fn human_assignment_fields_mut(&mut self) -> (&mut WorldParameters, &mut [u8]) {
        (&mut self.parameters, &mut self.parameter_modified68)
    }
    ///Native Session label setters mutate this same parameter storage. This
    ///loan does not expose a constructor or allow replacing a Creator owner.
    pub(crate) fn label_fields_mut(
        &mut self,
    ) -> (&mut WorldParameters, &mut crate::creator_labels::State) {
        (&mut self.parameters, &mut self.labels)
    }
    pub fn labels(&self) -> &crate::creator_labels::State {
        &self.labels
    }
    pub fn extents_bits(&self) -> [u32; 2] {
        self.extents
    }
    pub fn rng_seed(&self) -> u32 {
        self.rng_seed
    }
    pub fn source_resource(&self) -> Option<&Arc<SourceResource>> {
        self.source_resource.as_ref()
    }
    ///Native diagnostic reads the retained pointer's Data+C registration
    ///ordinal. Validate its canonical source identity without re-translating
    ///the retained IDS or substituting a serialized string's position.
    pub(crate) fn diagnostic_reference_index(&self,catalog:&Catalog<'_>,manager:&str)->Result<u32,String> {
        let named=match manager {"Biome"=>self.biome.as_ref(),"Weather"=>self.weather.as_ref(),_=>return Err("Creator diagnostic reference kind differs".into())};
        let Some(named)=named else{return Ok(0)};
        let registry=catalog.actors.native_data_registry(manager).ok_or("Creator diagnostic manager absent")?;
        if !matches!(registry.mode,Some(0|1)) {return Err("Creator diagnostic predefined/unbound ordinal producer".into())}
        let index=registry.entries.iter().position(|e|e.ids==named.ids&&e.path==named.path&&e.source==named.source&&e.reloads.is_empty()).ok_or("Creator diagnostic canonical reference differs")?;
        catalog.actors.native_data_node(&registry.entries[index]).map_err(err)?;
        u32::try_from(index).map_err(err)
    }
    /// The current base512A00 payload only. A derived Load serializer must
    /// write its own version0 BEFORE this payload and its SAME source+50 IDS
    /// AFTER it (512C30). This DTO never claims to be a complete derived stream.
    pub fn base_payload(&self) -> WorldCreator {
        WorldCreator {
            class_name: "KKC_WorldCreator".encode_utf16().collect(),
            version: 3,
            rules: self.rules.to_serialized(),
            parameters: self.parameters.clone(),
            world_extents_bits: self.extents,
            biome: self
                .biome
                .as_ref()
                .map(|v| v.ids.encode_utf16().collect())
                .unwrap_or_default(),
            weather: self
                .weather
                .as_ref()
                .map(|v| v.ids.encode_utf16().collect())
                .unwrap_or_default(),
            rng_seed: self.rng_seed,
        }
    }
}

#[cfg(test)]
#[path = "resource_creator_tests.rs"]
mod tests;

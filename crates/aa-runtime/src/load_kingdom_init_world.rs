//! Saved Team4E68C0 and Kingdom4E5DF0 over the actual held load allocation.
//! Base Actor/Object services use the same Record and persistent load world.
//! No actor/flow/list payload is cloned and no missing producer succeeds.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::{ActorId, loading::LoadKind};
use aa_economy::{EconomyError, kingdom_init as native};
use aa_mapworld::{
    restore::{NativeMessage, RestoreDefinitions, RestoredRecord},
    special_state::SpecialShell,
};
use aa_numeric::Precision;
use serde::Serialize;

#[cfg(test)]
#[path = "load_kingdom_init_world_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;
fn error(value: impl std::fmt::Display) -> EconomyError {
    EconomyError::World(value.to_string())
}
fn word(shell: &SpecialShell, offset: u32) -> Result<u32> {
    shell
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| format!("Kingdom Init requires actual derived+{offset:X}"))
}

/// One authoritative GWorld+144/+148/+154/+160 owner. Saved slot counts are
/// initialized before loading the first Kingdom; source slot traversal never
/// reconstructs these arrays from GTIDs afterward.
#[derive(Debug, Serialize)]
pub struct KingdomRegistryState {
    count: u32,
    all: Vec<Option<ActorId>>,
    majors: Vec<Option<ActorId>>,
    color_counts: Option<Vec<u32>>,
}
impl KingdomRegistryState {
    ///4F2949..297E initializes World+160 by430240 with GameData+3F4
    /// (KingdomColor registry count), before4FC600 initializes the two saved
    /// Kingdom vectors from their serialized counts. These are null/zero
    /// constructors, not inferred ownership reconstructed from source records.
    pub fn from_inventory(
        catalog: &Catalog<'_>,
        inventory: &aa_mapworld::WorldInventory,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        let source = inventory
            .kingdoms
            .as_ref()
            .ok_or("KINGDOMS counts have not been decoded")?;
        let colors = catalog
            .actors
            .native_data_registry("KingdomColor")
            .ok_or("native KingdomColor registration owner is absent")?;
        let mut counts = Vec::new();
        counts
            .try_reserve_exact(colors.entries.len())
            .map_err(|e| e.to_string())?;
        counts.resize(colors.entries.len(), 0);
        let mut state = Self::from_saved_counts(
            u32::try_from(source.slots.len()).map_err(|e| e.to_string())?,
            source.major_count,
        )?;
        state.color_counts = Some(counts);
        Ok(state)
    }
    pub fn from_saved_counts(all_count: u32, major_count: u32) -> Result<Self> {
        fn slots(count: u32) -> Result<Vec<Option<ActorId>>> {
            if count > i32::MAX as u32 {
                return Err(
                    "Kingdom saved vector count is negative in the native signed domain".into(),
                );
            }
            let mut values = Vec::new();
            values
                .try_reserve_exact(count as usize)
                .map_err(|e| e.to_string())?;
            values.resize(count as usize, None);
            Ok(values)
        }
        Ok(Self {
            count: 0,
            all: slots(all_count)?,
            majors: slots(major_count)?,
            color_counts: None,
        })
    }
    pub fn all(&self) -> &[Option<ActorId>] {
        &self.all
    }
    pub fn majors(&self) -> &[Option<ActorId>] {
        &self.majors
    }
    pub fn count(&self) -> u32 {
        self.count
    }
    pub fn color_counts(&self) -> Option<&[u32]> {
        self.color_counts.as_deref()
    }
    ///4F09C0's ordered World mutation, before actual76C7C0 notification.
    ///World144 is a mutation counter: removal increments it too.
    pub(crate) fn unregister_4f09c0(&mut self,id:ActorId,shell:&mut SpecialShell,color:Option<u32>)->Result<()> {
        let all=word(shell,0x188)? as i32;
        let major=word(shell,0x18c)? as i32;
        //Portable ownership guards; they do not emulate invalid native pointers.
        if all>=0 && self.all.get(all as usize)!=Some(&Some(id)) {
            return Err("4F09C0 actual all-Kingdom slot owner differs".into());
        }
        if major>=0 && self.majors.get(major as usize)!=Some(&Some(id)) {
            return Err("4F09C0 actual major-Kingdom slot owner differs".into());
        }
        if let Some(index)=color {
            if self.color_counts.as_ref().and_then(|v|v.get(index as usize)).is_none() {
                return Err("4F09C0 actual Color+C owner exceeds World160".into());
            }
        }
        self.count=self.count.wrapping_add(1);
        if let Some(index)=color {
            let value=&mut self.color_counts.as_mut().unwrap()[index as usize];
            *value=value.wrapping_sub(1);
        }
        if all>=0 {
            self.all[all as usize]=None;
            *shell.words.get_mut(&0x188).unwrap()=u32::MAX;
        }
        if major>=0 {
            self.majors[major as usize]=None;
            *shell.words.get_mut(&0x18c).unwrap()=u32::MAX;
        }
        Ok(())
    }
    ///4F0D00 updates the SAME World+160 allocation before4E0ED0 stores
    ///the Kingdom's new Color pointer. Repeated/same-color writes retain the
    ///native decrement-then-increment order and wrapping DWORD arithmetic.
    pub(crate) fn transition_color(&mut self,old:Option<u32>,new:Option<u32>)->Result<()> {
        if let Some(index)=old {
            let value=self.color_counts.as_mut().ok_or("actual color-count owner absent")?
                .get_mut(index as usize).ok_or("old Color+C exceeds actual World+160")?;
            *value=value.wrapping_sub(1);
        }
        if let Some(index)=new {
            let value=self.color_counts.as_mut().ok_or("actual color-count owner absent")?
                .get_mut(index as usize).ok_or("new Color+C exceeds actual World+160")?;
            *value=value.wrapping_add(1);
        }
        Ok(())
    }
    /// Move the actual constructed color-count allocation into this owner.
    /// This does not infer values from a Color domain length or Kingdom scan.
    pub fn bind_color_counts(&mut self, values: Vec<u32>) -> Result<()> {
        if self.color_counts.is_some() {
            return Err("GWorld+160 color counts already have an owner".into());
        }
        self.color_counts = Some(values);
        Ok(())
    }
    fn insert_slot(
        slots: &mut Vec<Option<ActorId>>,
        index: u32,
        kingdom: ActorId,
        label: &str,
    ) -> Result<()> {
        if index > i32::MAX as u32 {
            return Err(format!(
                "4F8AF0 negative {label} slot would dereference before its allocation"
            ));
        }
        let index = index as usize;
        if slots.get(index).is_some_and(Option::is_some) {
            return Err(format!("4F8AF0 Loading Exception: occupied {label} slot"));
        }
        if index >= slots.len() {
            if index > slots.len() {
                return Err(
                    "4F8AF0 sparse growth requires owned uninitialized vector slots".into(),
                );
            }
            let count = index.checked_add(1).ok_or("4F8AF0 vector size overflow")?;
            slots
                .try_reserve_exact(count - slots.len())
                .map_err(|e| e.to_string())?;
            slots.resize(count, None);
        }
        slots[index] = Some(kingdom);
        Ok(())
    }
    ///4F8AF0 in native store order; a later failure keeps earlier effects.
    pub fn register(&mut self, kingdom: ActorId, derived: &SpecialShell) -> Result<()> {
        self.register_with_color(kingdom, derived, |color| {
            color
                .index
                .ok_or("4F8AF0 Color+C has no verified source ordinal".into())
        })
    }
    fn register_with_color(
        &mut self,
        kingdom: ActorId,
        derived: &SpecialShell,
        color_index: impl FnOnce(&aa_mapworld::restore::NamedDefinition) -> Result<u32>,
    ) -> Result<()> {
        self.count = self.count.wrapping_add(1);
        Self::insert_slot(&mut self.all, word(derived, 0x188)?, kingdom, "all-Kingdom")?;
        if word(derived, 0x190)? == 0 {
            Self::insert_slot(
                &mut self.majors,
                word(derived, 0x18c)?,
                kingdom,
                "major-Kingdom",
            )?;
        }
        let color = match derived.references.get(&0x1e8) {
            Some(value) => value.as_ref(),
            None if word(derived, 0x1e8)? == 0 => None,
            None => return Err("4F8AF0 nonnull KingdomColor source owner is unavailable".into()),
        };
        if let Some(color) = color {
            let index = color_index(color)?;
            let cell = self
                .color_counts
                .as_mut()
                .ok_or("4F8AF0 actual GWorld+160 color-count allocation is unbound")?
                .get_mut(index as usize)
                .ok_or("4F8AF0 Color+C is outside actual color-count storage")?;
            *cell = cell.wrapping_add(1);
        }
        Ok(())
    }
}

///47A650's create path stores the current KingdomColor manager+28 at the
///new Data+C, then93D4E0 appends that same allocation. Reloads reuse Data+C.
///This reads the canonical reference already resolved by the load; it does
///not translate its IDS again or write a second mutable ordinal field.
fn current_color_index(
    catalog: &Catalog<'_>,
    color: &aa_mapworld::restore::NamedDefinition,
) -> Result<u32> {
    catalog.require_source_owner()?;
    let registry = catalog
        .actors
        .native_data_registry("KingdomColor")
        .ok_or("4F8AF0 actual KingdomColor source registry is absent")?;
    let mut matches = registry
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.ids.eq_ignore_ascii_case(&color.id));
    let (index, _) = matches
        .next()
        .ok_or("4F8AF0 current Color source identity is unavailable")?;
    if matches.next().is_some() {
        return Err("4F8AF0 Color source identity is ambiguous".into());
    }
    let index = u32::try_from(index).map_err(|e| e.to_string())?;
    if color.index.is_some_and(|existing| existing != index) {
        return Err("4F8AF0 Color ordinal differs from its actual source allocation".into());
    }
    Ok(index)
}

/// Borrows of the same persistent load owners. Restriction profiles are actual
///505600 data; None is a reached error, never an unrestricted profile manager.
pub struct Context<'a, S> {
    pub actor_services: &'a mut S,
    pub world_version: &'a mut u32,
    pub registry_state: &'a mut KingdomRegistryState,
    pub restrictions: Option<&'a crate::restriction_groups::LoadedRestrictionGroups>,
    pub messages: &'a mut Vec<NativeMessage>,
    pub source_ordinal: usize,
}

struct World<'a, 'r, 'source, S> {
    catalog: &'a Catalog<'source>,
    record: &'a mut Record<'r>,
    derived: &'a mut SpecialShell,
    context: Context<'a, S>,
    source_pc: Precision,
    // Native category nodes are distinct from definition tokens. These are
    // actual registration-order arena positions, traversed head first.
    categories: Option<[Vec<usize>; 2]>,
}
impl<S> World<'_, '_, '_, S> {
    fn owner(&self, raw: u32) -> Result<ActorId> {
        let id = ActorId::from_raw(raw);
        if id != self.record.owner {
            return Err("Kingdom Init attempted another held allocation".into());
        }
        Ok(id)
    }
    fn special(&self, id: ActorId) -> Result<&SpecialShell> {
        if id == self.record.owner {
            return Ok(self.derived);
        }
        let record = &self
            .record
            .registry
            .get(id)
            .ok_or("Kingdom Init reference is stale")?
            .value;
        match record {
            RestoredRecord::Kingdom(value) => value
                .runtime
                .as_ref()
                .and_then(|r| r.derived.as_ref())
                .ok_or("referenced Kingdom shell is unconstructed".into()),
            RestoredRecord::Team(value) => value
                .runtime
                .as_ref()
                .and_then(|r| r.derived.as_ref())
                .ok_or("referenced Team shell is unconstructed".into()),
            _ => Err("Kingdom Init reference is not a current Team/Kingdom".into()),
        }
    }
    fn special_mut(&mut self, id: ActorId) -> Result<&mut SpecialShell> {
        if id == self.record.owner {
            return Ok(self.derived);
        }
        let record = self
            .record
            .registry
            .value_mut(id)
            .ok_or("Kingdom Init reference is stale")?;
        match record {
            RestoredRecord::Kingdom(value) => value
                .runtime
                .as_mut()
                .and_then(|r| r.derived.as_mut())
                .ok_or("referenced Kingdom shell is unconstructed".into()),
            RestoredRecord::Team(value) => value
                .runtime
                .as_mut()
                .and_then(|r| r.derived.as_mut())
                .ok_or("referenced Team shell is unconstructed".into()),
            _ => Err("Kingdom Init reference is not a current Team/Kingdom".into()),
        }
    }
    fn team_id(&self, id: ActorId) -> Result<Option<ActorId>> {
        let shell = self.special(id)?;
        match shell.identity_references.get(&0x1ec) {
            Some(value) => Ok(*value),
            None if word(shell, 0x1ec)? == 0 => Ok(None),
            None => Err("Kingdom+1EC current Team owner is absent".into()),
        }
    }
    fn vector(&self, id: ActorId, offset: u32, index: u32) -> Result<u32> {
        self.special(id)?
            .vectors
            .get(&offset)
            .ok_or("relation vector owner is missing")?
            .values
            .get(index as usize)
            .copied()
            .ok_or("relation vector native slot is missing".into())
    }
    fn relation(&self, id: ActorId, other: ActorId) -> Result<u32> {
        if id == other {
            return Ok(0);
        }
        let current_team = self.team_id(id)?;
        let target_team = self.team_id(other)?;
        let target = if let Some(team) = target_team {
            (team, 0x1dc, 0x58)
        } else {
            (other, 0x1d0, 0x4c)
        };
        let index = word(
            self.special(target.0)?,
            if target_team.is_some() { 0x38 } else { 0x188 },
        )?;
        match current_team {
            Some(team) => self.vector(team, target.2, index),
            None => self.vector(id, target.1, index),
        }
    }
    fn related_actual(&self, id: ActorId, other: ActorId) -> Result<bool> {
        //4E5250 virtual+138 is self OR relation==1, not +140's ==2.
        Ok(id == other || self.relation(id, other)? == 1)
    }
    fn refresh_mask(&mut self, id: ActorId, supply: bool) -> Result<()> {
        let offset = if supply { 0x22c } else { 0x228 };
        if supply {
            let shell = self.special_mut(id)?;
            //559600 writes a WORD and preserves adjacent+22E/+22F bytes.
            if let Some(word) = shell.words.get_mut(&0x22c) {
                *word &= 0xffff0000;
            } else {
                if !shell.bytes.contains_key(&0x22c) || !shell.bytes.contains_key(&0x22d) {
                    return Err("Kingdom+22C actual word byte storage is absent".into());
                }
                shell.bytes.insert(0x22c, 0);
                shell.bytes.insert(0x22d, 0);
            }
        } else {
            *self
                .special_mut(id)?
                .words
                .get_mut(&offset)
                .ok_or("Kingdom+228 actual word storage is absent")? = 0;
        }
        if word(self.special(id)?, 0x190)? != 0 {
            return Ok(());
        }
        let mut index = 0;
        while index < self.context.registry_state.majors.len() {
            if let Some(other) = self.context.registry_state.majors[index] {
                if self.related_actual(id, other)? && word(self.special(other)?, 0x190)? == 0 {
                    let major = word(self.special(other)?, 0x18c)?;
                    if supply {
                        let bit = 1u32.wrapping_shl(major & 31) as u16;
                        let shell = self.special_mut(id)?;
                        if let Some(word) = shell.words.get_mut(&offset) {
                            *word = (*word & 0xffff0000) | u32::from((*word as u16) | bit)
                        } else {
                            let low = *shell
                                .bytes
                                .get(&0x22c)
                                .ok_or("Kingdom supply low byte absent")?;
                            let high = *shell
                                .bytes
                                .get(&0x22d)
                                .ok_or("Kingdom supply high byte absent")?;
                            let value = u16::from_le_bytes([low, high]) | bit;
                            let bytes = value.to_le_bytes();
                            shell.bytes.insert(0x22c, bytes[0]);
                            shell.bytes.insert(0x22d, bytes[1]);
                        }
                    } else {
                        let value =
                            word(self.special(id)?, offset)? | 1u32.wrapping_shl(major & 31);
                        self.special_mut(id)?.words.insert(offset, value);
                        if word(self.special(other)?, 0x190)? == 0 {
                            let value = word(self.special(id)?, offset)?
                                | 1u32.wrapping_shl(
                                    major.wrapping_add(aa_zones::tile::VISIBLE_SHIFT) & 31,
                                );
                            self.special_mut(id)?.words.insert(offset, value);
                        }
                    }
                }
            }
            index += 1;
        }
        Ok(())
    }
}

/// Runtime source projection used only on the reached canonical definition.
/// The root provenance selects a registered owner without translating it again.
fn source_node<'a>(
    catalog: &'a Catalog<'_>,
    canonical: &str,
) -> Result<aa_rules::actors::ResolvedNode<'a>> {
    catalog.require_source_owner()?;
    let entry = catalog
        .actors
        .actors
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(canonical))
        .ok_or("canonical Kingdom recruit definition is missing")?;
    let mut found = None;
    for tag in aa_rules::actors::native_load::THING_TAGS {
        for root in catalog
            .actors
            .resolved_definitions(tag)
            .map_err(|e| e.to_string())?
        {
            if root.sources().first() == Some(&entry.source) {
                if found.is_some() {
                    return Err("canonical recruit source root is ambiguous".into());
                }
                found = Some(root)
            }
        }
    }
    found.ok_or("canonical recruit source provenance is unavailable".into())
}
fn source_text(
    node: &aa_rules::actors::ResolvedNode<'_>,
    field: &str,
    pc: Precision,
) -> Result<String> {
    match node.field(field, pc).map_err(|e| e.to_string())? {
        Some(aa_rules::native::TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!("Kingdom recruit source requires string {field}")),
    }
}

impl<S> World<'_, '_, '_, S> {
    fn canonical(&self, token: u32) -> Result<String> {
        let bank = self
            .catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| e.to_string())?;
        let entry = bank.entry(token).map_err(|e| e.to_string())?;
        if entry.key.domain != "actordefinition" {
            return Err("Kingdom Init definition token domain differs".into());
        }
        Ok(entry.key.name.clone())
    }
    fn token(&self, canonical: &str) -> Result<u32> {
        self.catalog
            .definition_bank
            .try_borrow_mut()
            .map_err(|e| e.to_string())?
            .intern("ActorDefinition", canonical)
    }
    fn categories(&mut self) -> Result<&[Vec<usize>; 2]> {
        if self.categories.is_none() {
            self.catalog.require_source_owner()?;
            if self
                .catalog
                .actors
                .native_data_registry("NameList")
                .is_none()
            {
                return Err(
                    "Kingdom Init category lists require native source registration order".into(),
                );
            }
            let mut roots = Vec::new();
            for tag in aa_rules::actors::native_load::THING_TAGS {
                roots.extend(
                    self.catalog
                        .actors
                        .resolved_definitions(tag)
                        .map_err(|e| e.to_string())?,
                )
            }
            if roots.len() != self.catalog.actors.actors.len() {
                return Err("Kingdom Init source registry projection is incomplete".into());
            }
            let mut groups = [Vec::new(), Vec::new()];
            for (index, actor) in self.catalog.actors.actors.iter().enumerate().rev() {
                let mut matches = roots
                    .iter()
                    .filter(|root| root.sources().first() == Some(&actor.source));
                let root = matches
                    .next()
                    .ok_or("Kingdom Init registered source root missing")?;
                if matches.next().is_some()
                    || root.sources() != actor.template_chain
                    || source_text(root, "IDS", self.source_pc)? != actor.id
                {
                    return Err("Kingdom Init source identity/template chain differs".into());
                }
                let category = source_text(root, "category", self.source_pc)?;
                let category = aa_rules::native::CATEGORIES
                    .iter()
                    .find(|entry| entry.name.eq_ignore_ascii_case(&category))
                    .ok_or("Kingdom Init source category is unknown")?;
                if let Some(group) = native::RECRUIT_CATEGORIES
                    .iter()
                    .position(|&id| id == category.registry_index)
                {
                    groups[group].push(index)
                }
            }
            self.categories = Some(groups);
        }
        self.categories
            .as_ref()
            .ok_or("Kingdom category owner unavailable".into())
    }
    fn requirement_present(&self, id: &str, primary: bool) -> Result<bool> {
        let node = source_node(self.catalog, id)?;
        let element = node
            .child("ElementComponent")
            .map_err(|e| e.to_string())?
            .ok_or("Element ActorData+2C8 owner is absent")?;
        let field = if primary {
            "required_properties"
        } else {
            "required_structures"
        };
        let text = source_text(&element, field, self.source_pc)?;
        let tokens =
            aa_rules::abilities::required_property_tokens(&text).map_err(|e| e.to_string())?;
        let mut seen = Vec::new();
        if primary {
            let layout = self.catalog.layout_data(self.source_pc)?;
            let registered = self
                .catalog
                .actors
                .actors
                .iter()
                .find(|a| a.id.eq_ignore_ascii_case(id))
                .ok_or("canonical Element root missing")?;
            let file = self
                .catalog
                .actors
                .sources
                .iter()
                .position(|source| source.path.eq_ignore_ascii_case(&registered.source.path))
                .ok_or("Element source file missing")?;
            let at = aa_rules::native::layout_data::RootPosition {
                file,
                offset: registered.source.span.start,
            };
            for token in &tokens {
                let property = layout.find_property(token, Some(at)).ok_or_else(|| {
                    format!("4C5720 unknown required property {token:?} for {id}")
                })?;
                if seen.contains(&property) {
                    return Err(format!(
                        "4C5720 duplicate required property {token:?} for {id}"
                    ));
                }
                seen.push(property);
            }
        } else {
            let mut seen = Vec::new();
            for token in &tokens {
                let target = self.catalog.actors.find(token).ok_or_else(|| {
                    format!("4C54B0 unknown required structure {token:?} for {id}")
                })?;
                if self
                    .catalog
                    .canonical_component_mask(&target.id, self.source_pc)?
                    & (1 << 13)
                    == 0
                {
                    return Err(format!(
                        "4C54B0 required structure {token:?} is not a Structure"
                    ));
                }
                if seen.contains(&target.id) {
                    return Err(format!(
                        "4C54B0 duplicate required structure {token:?} for {id}"
                    ));
                }
                seen.push(target.id.clone());
            }
        }
        Ok(!tokens.is_empty())
    }
}
impl<S: crate::load_actor_init_world::Services> native::World for World<'_, '_, '_, S> {
    fn actor_init(
        &mut self,
        kingdom: u32,
        argument: u32,
        pc: aa_economy::X87Precision,
    ) -> native::Result<bool> {
        self.owner(kingdom).map_err(error)?;
        if argument != 0 {
            return Err(error("saved Kingdom Init requires actual NULL argument"));
        }
        let pc = match pc {
            aa_economy::X87Precision::Single24 => Precision::Single24,
            aa_economy::X87Precision::Double53 => Precision::Double53,
            aa_economy::X87Precision::Extended64 => Precision::Extended64,
        };
        crate::load_actor_init_world::initialize(
            self.catalog,
            self.record,
            pc,
            self.context.actor_services,
        )
        .map_err(error)
    }
    fn initialized_diagnostic(&mut self, kingdom: u32) -> native::Result<()> {
        self.owner(kingdom).map_err(error)?;
        let name = String::from_utf16(&self.record.shell.thing.name_utf16).map_err(error)?;
        self.context.messages.try_reserve(1).map_err(error)?;
        self.context.messages.push(NativeMessage {
            source_ordinal: self.context.source_ordinal,
            function: 0x404f60,
            line: 0x3c7,
            text: format!("GKingdom::Init() {name}"),
        });
        Ok(())
    }
    fn world_version(&mut self) -> native::Result<u32> {
        Ok(*self.context.world_version)
    }
    fn schedule_word_184(&mut self, kingdom: u32) -> native::Result<u32> {
        self.owner(kingdom).map_err(error)?;
        word(self.derived, 0x184).map_err(error)
    }
    fn legacy_schedule(&mut self, _: u32) -> native::Result<()> {
        Err(error(
            "50AC00 legacy Kingdom+180 scheduling owner is unbound",
        ))
    }
    fn resource_count(&mut self) -> native::Result<i32> {
        let count = self
            .catalog
            .resource_domain_count(self.source_pc)
            .map_err(error)?;
        i32::try_from(count).map_err(error)
    }
    fn clear_flow_cell(
        &mut self,
        kingdom: u32,
        flow: native::Flow,
        index: usize,
    ) -> native::Result<()> {
        self.owner(kingdom).map_err(error)?;
        let vector = self
            .derived
            .vectors
            .get_mut(&flow.offset())
            .ok_or_else(|| error("Kingdom flow allocation is unconstructed"))?;
        if index >= vector.values.len() {
            let count = index
                .checked_add(1)
                .ok_or_else(|| error("Kingdom flow index overflow"))?;
            let count_word = u32::try_from(count).map_err(error)?;
            let mut capacity = vector.capacity_hint;
            if count_word > capacity {
                let twice = vector
                    .capacity_hint
                    .checked_mul(2)
                    .ok_or_else(|| error("Kingdom flow capacity arithmetic overflow"))?;
                capacity = count_word.max(twice);
                if vector.capacity_hint != 0 {
                    capacity = capacity.max(4);
                }
            }
            vector
                .values
                .try_reserve_exact((capacity as usize).saturating_sub(vector.values.len()))
                .map_err(error)?;
            vector.capacity_hint = capacity;
            vector.values.resize(count, 0);
        }
        vector.values[index] = 0;
        Ok(())
    }
    fn team(&mut self, kingdom: u32) -> native::Result<Option<u32>> {
        let owner = self.owner(kingdom).map_err(error)?;
        Ok(self.team_id(owner).map_err(error)?.map(ActorId::raw))
    }
    fn prepend_team_member(&mut self, team: u32, kingdom: u32) -> native::Result<()> {
        let owner = self.owner(kingdom).map_err(error)?;
        let list = self
            .special_mut(ActorId::from_raw(team))
            .map_err(error)?
            .team_members
            .as_mut()
            .ok_or_else(|| error("Team+44/+48 actual list owner is absent"))?;
        list.try_reserve(1).map_err(error)?;
        list.insert(0, owner);
        Ok(())
    }
    fn team_member_count(&mut self, team: u32) -> native::Result<u32> {
        word(self.special(ActorId::from_raw(team)).map_err(error)?, 0x68).map_err(error)
    }
    fn set_team_member_count(&mut self, team: u32, value: u32) -> native::Result<()> {
        let count = self
            .special_mut(ActorId::from_raw(team))
            .map_err(error)?
            .words
            .get_mut(&0x68)
            .ok_or_else(|| error("Team+68 actual count is absent"))?;
        *count = value;
        Ok(())
    }
    fn definition_category_head(&mut self, category: u32) -> native::Result<Option<u32>> {
        let group = native::RECRUIT_CATEGORIES
            .iter()
            .position(|&c| c == category)
            .ok_or_else(|| error("unexpected Kingdom recruit category"))?;
        self.categories().map_err(error)?[group]
            .first()
            .map(|&i| u32::try_from(i + 1).map_err(error))
            .transpose()
    }
    fn definition_at_node(&mut self, node: u32) -> native::Result<u32> {
        let index = node
            .checked_sub(1)
            .ok_or_else(|| error("null category-list node"))? as usize;
        let canonical = &self
            .catalog
            .actors
            .actors
            .get(index)
            .ok_or_else(|| error("category-list node outside source owner"))?
            .id;
        self.token(canonical).map_err(error)
    }
    fn next_definition_node(&mut self, node: u32) -> native::Result<Option<u32>> {
        let index = node
            .checked_sub(1)
            .ok_or_else(|| error("null category-list node"))? as usize;
        for group in self.categories().map_err(error)? {
            if let Some(at) = group.iter().position(|&i| i == index) {
                return group
                    .get(at + 1)
                    .map(|&i| u32::try_from(i + 1).map_err(error))
                    .transpose();
            }
        }
        Err(error(
            "category-list cursor does not belong to current source owner",
        ))
    }
    fn definition_component_mask(&mut self, definition: u32) -> native::Result<u32> {
        self.catalog
            .canonical_component_mask(&self.canonical(definition).map_err(error)?, self.source_pc)
            .map_err(error)
    }
    fn recruit_link_nonempty(
        &mut self,
        definition: u32,
        link: native::RecruitLink,
    ) -> native::Result<bool> {
        let id = self.canonical(definition).map_err(error)?;
        match link {
            native::RecruitLink::Primary => self.requirement_present(&id, true).map_err(error),
            native::RecruitLink::Secondary => self.requirement_present(&id, false).map_err(error),
            native::RecruitLink::Property17c => self
                .catalog
                .native_recruit_backrefs(self.source_pc)
                .and_then(|data| data.present(&id).map_err(|e| e.to_string()))
                .map_err(error),
        }
    }
    fn recruit_allowed(&mut self, kingdom: u32, definition: u32) -> native::Result<bool> {
        let id = self.owner(kingdom).map_err(error)?;
        let canonical = self.canonical(definition).map_err(error)?;
        if let Some(team) = self.team_id(id).map_err(error)? {
            let name = self
                .special(team)
                .map_err(error)?
                .strings_utf16
                .get(&0x64)
                .ok_or_else(|| error("Team+64 restriction profile string is absent"))?;
            let name = String::from_utf16(name).map_err(error)?;
            let loaded = self.context.restrictions.ok_or_else(|| {
                error("4E1810 actual loaded RestrictionGroups owner is unavailable")
            })?;
            loaded.require_catalog(self.catalog).map_err(error)?;
            let profiles = loaded.profiles();
            if !aa_economy::restrictions::actor_allowed(profiles.find(&name), None, &canonical) {
                return Ok(false);
            }
        }
        let name = self
            .derived
            .strings_utf16
            .get(&0x348)
            .ok_or_else(|| error("Kingdom+348 restriction profile string is absent"))?;
        let name = String::from_utf16(name).map_err(error)?;
        let loaded = self
            .context
            .restrictions
            .ok_or_else(|| error("4E1810 actual loaded RestrictionGroups owner is unavailable"))?;
        loaded.require_catalog(self.catalog).map_err(error)?;
        let profiles = loaded.profiles();
        Ok(aa_economy::restrictions::actor_allowed(
            None,
            profiles.find(&name),
            &canonical,
        ))
    }
    fn append_plain_recruit(
        &mut self,
        kingdom: u32,
        item: native::PlainRecruit,
    ) -> native::Result<()> {
        self.owner(kingdom).map_err(error)?;
        if item.available != 1 {
            return Err(error("Kingdom Init plain recruit constructor byte differs"));
        }
        self.derived
            .recruit_items
            .reserve_entries(1)
            .map_err(error)?;
        self.derived.recruit_items.add_plain(item.definition);
        Ok(())
    }
    fn kingdom_word_190(&mut self, kingdom: u32) -> native::Result<u32> {
        word(
            self.special(ActorId::from_raw(kingdom)).map_err(error)?,
            0x190,
        )
        .map_err(error)
    }
    fn sai_active(&mut self) -> native::Result<bool> {
        self.catalog.saved_observer_active().map_err(error)
    }
    fn register_sai(&mut self, _: u32) -> native::Result<()> {
        Err(error("79E170 active SAI Kingdom registration is unbound"))
    }
    fn register_world(&mut self, kingdom: u32) -> native::Result<()> {
        let id = self.owner(kingdom).map_err(error)?;
        self.context
            .registry_state
            .register_with_color(id, self.derived, |color| {
                current_color_index(self.catalog, color)
            })
            .map_err(error)
    }
    fn major_count(&mut self) -> native::Result<i32> {
        i32::try_from(self.context.registry_state.majors.len()).map_err(error)
    }
    fn major_at(&mut self, index: usize) -> native::Result<Option<u32>> {
        self.context
            .registry_state
            .majors
            .get(index)
            .copied()
            .ok_or_else(|| error("major vector slot is unavailable"))
            .map(|v| v.map(ActorId::raw))
    }
    fn related(&mut self, other: u32, kingdom: u32) -> native::Result<bool> {
        self.related_actual(ActorId::from_raw(other), ActorId::from_raw(kingdom))
            .map_err(error)
    }
    fn refresh_relation_228(&mut self, other: u32) -> native::Result<()> {
        self.refresh_mask(ActorId::from_raw(other), false)
            .map_err(error)
    }
    fn refresh_relation_22c(&mut self, other: u32) -> native::Result<()> {
        self.refresh_mask(ActorId::from_raw(other), true)
            .map_err(error)
    }
}

pub fn initialize_kingdom<S: crate::load_actor_init_world::Services>(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    derived: &mut SpecialShell,
    context: Context<'_, S>,
    pc: Precision,
) -> Result<bool> {
    if record.definition.native_class != 4
        || record.shell.kind != LoadKind::Actor
        || record.shell.actor.is_none()
        || derived.recruitment_owner != Some(record.owner)
    {
        return Err("Kingdom Init actual held class/constructor owner differs".into());
    }
    if !record
        .registry
        .get(record.owner)
        .is_some_and(|e| matches!(e.value, RestoredRecord::Loading))
    {
        return Err("Kingdom Init requires the registered held allocation".into());
    }
    let source_pc = {
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| e.to_string())?;
        let source = bank.combat_source()?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err("Kingdom Init source FP owner differs from Catalog".into());
        }
        source.source_precision()
    };
    let owner = record.owner.raw();
    let precision = match pc {
        Precision::Single24 => aa_economy::X87Precision::Single24,
        Precision::Double53 => aa_economy::X87Precision::Double53,
        Precision::Extended64 => aa_economy::X87Precision::Extended64,
    };
    native::initialize(
        owner,
        0,
        precision,
        &mut World {
            catalog,
            record,
            derived,
            context,
            source_pc,
            categories: None,
        },
    )
    .map_err(|e| e.to_string())
}

struct TeamWorld<'a, 'record, 'world, 'globals, 'assets> {
    catalog: &'a Catalog<'assets>,
    record: &'a mut Record<'record>,
    context: &'a mut crate::load_object_init_world::Context<'world, 'globals>,
    messages: &'a mut Vec<NativeMessage>,
    source_ordinal: usize,
}
fn team_error(value: impl std::fmt::Display) -> aa_mapworld::team_init::Error {
    aa_mapworld::team_init::Error::World(value.to_string())
}
impl TeamWorld<'_, '_, '_, '_, '_> {
    fn owner(&self, id: ActorId) -> aa_mapworld::team_init::Result<()> {
        if id != self.record.owner {
            return Err(team_error("Team Init attempted another held allocation"));
        }
        Ok(())
    }
}
impl aa_mapworld::team_init::World for TeamWorld<'_, '_, '_, '_, '_> {
    fn flags(&self, id: ActorId) -> aa_mapworld::team_init::Result<u32> {
        self.owner(id)?;
        Ok(self.record.shell.thing.flags)
    }
    fn tile_size_bits(&self) -> aa_mapworld::team_init::Result<u32> {
        Ok(self.context.terrain.grid.cell_size)
    }
    fn snap_to_grid(
        &mut self,
        id: ActorId,
        tile: u32,
        pc: Precision,
    ) -> aa_mapworld::team_init::Result<()> {
        self.owner(id)?;
        let xy: &mut [u32; 2] = (&mut self.record.shell.thing.position_bits[..2])
            .try_into()
            .map_err(|_| team_error("Team position allocation differs"))?;
        aa_spatial::snap_half_cell(xy, tile, pc).map_err(team_error)
    }
    fn check_placement_null(
        &mut self,
        id: ActorId,
        pc: Precision,
    ) -> aa_mapworld::team_init::Result<bool> {
        self.owner(id)?;
        let code = crate::load_object_init_world::placement_null(
            self.catalog,
            self.record,
            self.context,
            pc,
        )
        .map_err(team_error)?;
        if code != 0 {
            return Err(team_error(format!(
                "4EF8B4 Team placement-failure diagnostic/map-warning owner required for code {code}"
            )));
        }
        Ok(true)
    }
    fn update_index_membership(&mut self, id: ActorId) -> aa_mapworld::team_init::Result<()> {
        self.owner(id)?;
        //4EE4C0: Team virtual+B0=false. Its flag branch still matters; only
        //unchanged membership is a native leaf. Reached Thing insertion or
        //removal must use the actual spatial descriptor owner.
        let flags = self.record.shell.thing.flags;
        let wants = flags & 0x4408 != 0;
        let current = flags & (1 << 26) != 0;
        if wants != current {
            crate::load_object_init_world::spatial_membership(
                self.catalog,
                self.record,
                self.context,
            )
            .map_err(team_error)?;
        }
        Ok(())
    }
    fn position_bits(&self, id: ActorId) -> aa_mapworld::team_init::Result<[u32; 3]> {
        self.owner(id)?;
        Ok(self.record.shell.thing.position_bits)
    }
    fn terrain_height(
        &mut self,
        position: [u32; 3],
        flags: u32,
        pc: Precision,
    ) -> aa_mapworld::team_init::Result<aa_numeric::Binary> {
        self.context
            .terrain
            .surface()
            .map_err(team_error)?
            .sample([position[0], position[1]], flags, pc)
            .map_err(team_error)
    }
    fn set_height_bits(&mut self, id: ActorId, bits: u32) -> aa_mapworld::team_init::Result<()> {
        self.owner(id)?;
        self.record.shell.thing.position_bits[2] = bits;
        Ok(())
    }
    fn name_utf16(&self, id: ActorId) -> aa_mapworld::team_init::Result<&[u16]> {
        self.owner(id)?;
        Ok(&self.record.shell.thing.name_utf16)
    }
    fn log_team_initialized(
        &mut self,
        id: ActorId,
        name: &[u16],
    ) -> aa_mapworld::team_init::Result<()> {
        self.owner(id)?;
        let name = String::from_utf16(name).map_err(team_error)?;
        self.messages.try_reserve(1).map_err(team_error)?;
        self.messages.push(NativeMessage {
            source_ordinal: self.source_ordinal,
            function: 0x404f60,
            line: 0xee5,
            text: format!("KKC_GTeam::Init() {name}"),
        });
        Ok(())
    }
}

pub fn initialize_team(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut crate::load_object_init_world::Context<'_, '_>,
    messages: &mut Vec<NativeMessage>,
    source_ordinal: usize,
    pc: Precision,
) -> Result<bool> {
    catalog.require_source_owner()?;
    let proof = &record.definition.proof;
    let category = proof.category;
    let source = catalog
        .actors
        .actors
        .iter()
        .find(|a| a.id.eq_ignore_ascii_case(&proof.id))
        .ok_or("Team Init source owner is missing")?;
    if record.definition.native_class != 5
        || category.native_kind != 0
        || category.registry_index != 25
        || category.runtime_class != 5
        || record.shell.kind != LoadKind::Thing
        || record.shell.object.is_some()
        || record.shell.actor.is_some()
        || source.source != proof.source
        || source.template_chain != proof.template_chain
        || !crate::load_spatial::same_proof(&record.shell.thing.definition, proof)
        || !record
            .registry
            .get(record.owner)
            .is_some_and(|e| matches!(e.value, RestoredRecord::Loading))
    {
        return Err("Team Init actual constructor/registered source identity differs".into());
    }
    let source_pc = {
        let bank = catalog
            .definition_bank
            .try_borrow()
            .map_err(|e| e.to_string())?;
        let source = bank.combat_source()?;
        if source.source_identities() != catalog.actors.sources.as_slice() {
            return Err("Team Init source FP owner differs from Catalog".into());
        }
        source.source_precision()
    };
    if context.terrain.source_precision() != source_pc {
        return Err("Team terrain owner source precision differs from definition Bank".into());
    }
    let owner = record.owner;
    aa_mapworld::team_init::init_saved(
        &mut TeamWorld {
            catalog,
            record,
            context,
            messages,
            source_ordinal,
        },
        owner,
        pc,
    )
    .map_err(|e| e.to_string())
}

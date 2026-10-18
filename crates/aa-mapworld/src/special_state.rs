//! Exact derived saved-load constructor state for Team and Kingdom. Native
//! allocation addresses are replaced by owned storage; untouched fields remain
//! absent. Definition and actor handles are bound by the later Load stages.
use crate::{Error, Result};

///4E8679..4E86B8: unordered/nonpositive input becomes positive zero, then a
/// strictly smaller maximum replaces it. An unordered maximum is ignored.
pub fn clamp_resource_bits(value: u32, maximum: u32) -> u32 {
    let value = if f32::from_bits(value) > 0.0 {
        value
    } else {
        0
    };
    if f32::from_bits(maximum) < f32::from_bits(value) {
        maximum
    } else {
        value
    }
}
use aa_actors::ActorId;
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct NativeVectorState {
    pub values: Vec<u32>,
    /// Original allocated capacity metadata; this is not a gameplay limit.
    pub capacity_hint: u32,
}
impl NativeVectorState {
    ///766DE0 append, preserving its signed count/doubled-capacity selection.
    ///Portable values are identities owned by the caller's actual allocation
    ///directory, rather than independent copies of pointed-to native records.
    pub fn push(&mut self, value:u32)->Result<()> {
        let count=self.values.len().checked_add(1).ok_or(Error::Allocation)?;
        let count=u32::try_from(count).map_err(|_|Error::Allocation)?;
        if count>i32::MAX as u32 {return Err(Error::Format("native vector signed-count domain".into()))}
        let twice=self.capacity_hint.wrapping_mul(2);
        let capacity=if (count as i32)>(twice as i32){count}
            else if (count as i32)>(self.capacity_hint as i32){twice}
            else{self.capacity_hint};
        self.values.try_reserve_exact((capacity as usize).saturating_sub(self.values.len()))
            .map_err(|_|Error::Allocation)?;
        self.capacity_hint=capacity;
        self.values.push(value);
        Ok(())
    }
    /// Original4CD0F0/4849F0 bulk replacement. Capacity affects storage only;
    /// ordered duplicate values and every saved word are preserved.
    pub fn replace_values(&mut self, values: &[u32]) -> Result<()> {
        let count = u32::try_from(values.len()).map_err(|_| Error::Allocation)?;
        if count > i32::MAX as u32 {
            return Err(Error::Format(
                "native vector count exceeds signed range".into(),
            ));
        }
        let mut capacity = self.capacity_hint;
        if count > capacity {
            let twice = self
                .capacity_hint
                .checked_mul(2)
                .ok_or_else(|| Error::Format("native vector capacity overflow".into()))?;
            capacity = count.max(twice).max(4);
        }
        self.values
            .try_reserve_exact((capacity as usize).saturating_sub(self.values.len()))
            .map_err(|_| Error::Allocation)?;
        self.capacity_hint = capacity;
        self.values.clear();
        self.values.extend_from_slice(values);
        Ok(())
    }
    fn replace_counted(&mut self, values: &crate::actor_payload::CountedList<u32>) -> Result<()> {
        if (values.serialized_count as i32) < 0 {
            return Err(Error::Format(
                "native negative vector length cannot form a valid owned runtime vector".into(),
            ));
        }
        if values.serialized_count as usize != values.values.len() {
            return Err(Error::Format(
                "runtime vector count differs from decoded payload".into(),
            ));
        }
        self.replace_values(&values.values)
    }
    fn empty(capacity_hint: u32) -> Result<Self> {
        let mut values = Vec::new();
        values
            .try_reserve_exact(capacity_hint as usize)
            .map_err(|_| Error::Allocation)?;
        Ok(Self {
            values,
            capacity_hint,
        })
    }
    fn zeroed(count: u32, capacity: u32) -> Result<Self> {
        let mut value = Self::empty(capacity)?;
        value.values.resize(count as usize, 0);
        Ok(value)
    }
}
///4E5370 constructs eleven79E130 class head/tail lists at26C, plus the
/// separate flare list2F8/2FC. Process pointers are replaced by current GTIDs;
/// no independent mutable scalar head/tail mirrors remain.
#[derive(Clone, Debug)]
pub struct KingdomActorLists {
    pub by_class: Vec<Vec<ActorId>>,
    pub flares: Vec<ActorId>,
    ///4E3C70/4F1430, Kingdom30C/310 head/tail. These are borrowed current
    ///GTIDs, deduplicated on insert and removed by first matching value.
    pub city_ownership: Vec<ActorId>,
}
#[derive(Clone, Debug)]
pub struct SpecialShell {
    /// Initialized scalar words, keyed by original byte offset. Vector count
    /// and capacity live only in vectors, not duplicated in this word table.
    pub words: BTreeMap<u32, u32>,
    pub bytes: BTreeMap<u32, u8>,
    pub strings_utf16: BTreeMap<u32, Vec<u16>>,
    pub vectors: BTreeMap<u32, NativeVectorState>,
    pub indexed_words: BTreeMap<u32, Vec<u32>>,
    pub recruitment_owner: Option<ActorId>,
    /// Canonical source definition identities replace process pointers.
    pub references: BTreeMap<u32, Option<crate::restore::NamedDefinition>>,
    pub identity_references: BTreeMap<u32, Option<ActorId>>,
    pub commander: Option<aa_combat::commander::State>,
    /// Count mismatch is a native diagnostic; discarded saved abilities are
    /// never described as restored. The constructed ability states remain.
    pub commander_load: Option<aa_combat::commander::LoadOutcome>,
    pub tactical: Option<crate::kingdom_ai::TacticalState>,
    pub strategic: Option<crate::kingdom_ai::StrategicState>,
    pub definition_counts: Option<crate::definition_counts::DefinitionCounts>,
    /// Native head-first list at36C. Values index the Nation Badge texture list.
    pub nation_badge_indices: Vec<u32>,
    pub missing_count_definitions: Vec<Vec<u16>>,
    pub invalid_nation_indices: Vec<u32>,
    /// Kingdom +264/+268 supply-source list, head first (4E1F00 head insert;
    /// the tail is the oldest entry). Empty means both words are null.
    pub supply_sources: Vec<ActorId>,
    /// Kingdom +1FC recruit item table (singles vector +214, change counter
    /// +224), keys = canonical ActorDefinition tokens. Restored from the
    /// saved Recruitment block; every shipped map saves it empty.
    pub recruit_items: aa_economy::kingdom_registry::RecruitItemTable,
    /// Team+44/+48 native head/tail list, head first with duplicates intact.
    /// None identifies a non-Team shell; no independently mutable raw pointer
    /// words mirror this owner. Team+68 remains its separate native scalar.
    pub team_members: Option<Vec<ActorId>>,
    pub actor_lists: Option<KingdomActorLists>,
}
impl SpecialShell {
    /// 4E1F00 (via thunk 4E38A0) from the Supply constructor 58AA50.
    pub fn register_supply_source(&mut self, actor: ActorId) -> bool {
        aa_actors::supply_state::register_source(&mut self.supply_sources, actor)
    }
    /// Kingdom +35C table (constructed empty by the Kingdom constructor).
    pub fn definition_counts_mut(&mut self) -> &mut crate::definition_counts::DefinitionCounts {
        if self.definition_counts.is_none() {
            self.indexed_words.remove(&0x35c);
            self.words.remove(&0x360);
            self.words.remove(&0x364);
            self.bytes.remove(&0x368);
        }
        self.definition_counts.get_or_insert_with(Default::default)
    }
    pub fn load_kingdom_definition_counts(
        &mut self,
        source: &crate::kingdoms::KingdomPayload,
        definitions: &impl crate::restore::RestoreDefinitions,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        if source.named_states_count.is_some_and(|v| (v as i32) > 0) {
            return Err(Error::Format(
                "nonempty Kingdom named states require native construction".into(),
            ));
        }
        if self.definition_counts.is_none() {
            self.definition_counts = Some(crate::definition_counts::DefinitionCounts::default());
            self.indexed_words.remove(&0x35c);
            self.words.remove(&0x360);
            self.words.remove(&0x364);
            self.bytes.remove(&0x368);
        }
        if let Some(records) = &source.references_35c {
            for record in &records.values {
                if record.reference_utf16.is_empty() {
                    continue;
                }
                let identity = definitions
                    .actor_definition_token(&record.reference_utf16, pc)
                    .map_err(Error::Format)?;
                if identity == 0 {
                    self.missing_count_definitions
                        .push(record.reference_utf16.clone());
                    continue;
                }
                self.definition_counts
                    .as_mut()
                    .ok_or_else(|| Error::Format("Kingdom count table disappeared".into()))?
                    .insert(identity, record.word)?;
            }
        }
        Ok(())
    }
    pub fn load_kingdom_final_fields(
        &mut self,
        source: &crate::kingdoms::KingdomPayload,
        definitions: &impl crate::restore::RestoreDefinitions,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        if let Some(indices) = source.words_36c.as_ref().filter(|v| !v.values.is_empty()) {
            let nation = self
                .references
                .get(&0x234)
                .and_then(Option::as_ref)
                .ok_or_else(|| {
                    Error::Format("Kingdom Nation indices have no resolved Nation".into())
                })?;
            let count = definitions
                .nation_badge_count(nation, pc)
                .map_err(Error::Format)?;
            for value in &indices.values {
                // Original4E925A uses signed comparison; negative saved values
                // are retained as list values and are not dereferenced here.
                if (*value as i32) >= (count as i32) {
                    self.invalid_nation_indices.push(*value);
                    continue;
                }
                self.nation_badge_indices
                    .try_reserve(1)
                    .map_err(|_| Error::Allocation)?;
                self.nation_badge_indices.insert(0, *value);
                self.words.remove(&0x36c);
                self.words.remove(&0x370);
            }
        }
        for (offset, value) in [
            (0x238, source.byte_238),
            (0x239, source.byte_239),
            (0x314, source.byte_314),
        ] {
            if let Some(value) = value {
                self.set_byte(offset, value);
            }
        }
        if let Some(records) = &source.resources_258 {
            let vector = self.vectors.get_mut(&0x258).ok_or_else(|| {
                Error::Format("Kingdom258 resource vector is unconstructed".into())
            })?;
            vector.values.fill(0);
            for record in &records.values {
                let definition = definitions
                    .resolve_named_definition("Resource", &record.reference_utf16, pc)
                    .map_err(Error::Format)?;
                if let Some(definition) = definition {
                    let index = definition.index.ok_or_else(|| {
                        Error::Format("Resource lookup lacks verified native ordinal".into())
                    })?;
                    if let Some(value) = vector.values.get_mut(index as usize) {
                        *value = record.word;
                    }
                }
            }
        }
        Ok(())
    }
    ///4E8D24..4E8EF8 constructs both owned states even for a non-AI Kingdom.
    pub fn load_kingdom_ai(&mut self, source: &crate::kingdoms::KingdomPayload) -> Result<()> {
        if self.tactical.is_none() {
            self.tactical = Some(crate::kingdom_ai::TacticalState::new()?);
            self.words.remove(&0x344);
        }
        if self.strategic.is_none() {
            self.strategic = Some(crate::kingdom_ai::StrategicState::new(true));
            self.words.remove(&0x330);
        }
        let Some(ai) = &source.ai else {
            return Ok(());
        };
        if let Some(tactical) = &ai.tactical {
            self.tactical
                .as_mut()
                .ok_or_else(|| Error::Format("tactical state disappeared".into()))?
                .apply_empty_saved(tactical)?;
        }
        if source.version >= 11 {
            self.strategic
                .as_mut()
                .ok_or_else(|| Error::Format("strategic state disappeared".into()))?
                .apply_saved(&ai.strategic)?;
        }
        // The +338 statistics snapshots (58F360 + 58F2E0) are economy state:
        // aa-runtime restoration::kingdom_economy loads them.
        if source.version == 10 {
            self.strategic
                .as_mut()
                .ok_or_else(|| Error::Format("strategic state disappeared".into()))?
                .apply_saved(&ai.strategic)?;
        }
        self.words.insert(0x334, ai.word_334);
        Ok(())
    }
    /// Native4E87E7..4E8D24. Nonempty ownership-bearing table records require
    /// their real factories; original tutorial records use the empty paths.
    pub fn load_kingdom_before_ai(
        &mut self,
        source: &crate::kingdoms::KingdomPayload,
        definitions: &impl crate::restore::RestoreDefinitions,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        if let Some(reference) = &source.nation_reference_utf16 {
            let nation = if reference.is_empty() {
                None
            } else {
                definitions
                    .resolve_named_definition("Nation", reference, pc)
                    .map_err(Error::Format)?
            };
            if nation.is_none() && !reference.is_empty() {
                return Err(Error::Format(
                    "native Nation lookup rejected unknown IDS".into(),
                ));
            }
            self.words.remove(&0x234);
            self.references.insert(0x234, nation);
        }
        if source
            .named_words
            .as_ref()
            .is_some_and(|v| !v.values.is_empty())
        {
            return Err(Error::Format(
                "nonempty Kingdom named-word table requires native entry construction".into(),
            ));
        }
        if let Some(value) = source.word_19c {
            self.words.insert(0x19c, value);
        }
        if source
            .references_24c
            .as_ref()
            .is_some_and(|v| !v.values.is_empty())
        {
            return Err(Error::Format(
                "Kingdom24C definition array requires native reference binding".into(),
            ));
        }
        if source
            .properties
            .as_ref()
            .is_some_and(|v| !v.values.is_empty())
        {
            return Err(Error::Format(
                "Kingdom properties require concrete effect construction and binding".into(),
            ));
        }
        if let Some(values) = source.bytes_23b_23c {
            self.set_byte(0x23b, values[0]);
            self.set_byte(0x23c, values[1]);
        }
        if let Some(value) = &source.string_240_utf16 {
            self.strings_utf16.insert(0x240, value.clone());
        }
        if let Some(value) = &source.string_348_utf16 {
            self.strings_utf16.insert(0x348, value.clone());
        }
        Ok(())
    }
    pub fn load_commander(
        &mut self,
        source: &crate::kingdoms::KingdomPayload,
        kingdom: u32,
        definitions: &impl crate::restore::RestoreDefinitions,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        let Some(source) = &source.commander else {
            return Ok(());
        };
        if source.presence == Some(0) {
            return Ok(());
        }
        let reference = source
            .reference_utf16
            .as_ref()
            .ok_or_else(|| Error::Format("present Commander lacks source reference".into()))?;
        let definition = definitions
            .resolve_commander(reference, pc)
            .map_err(Error::Format)?;
        self.words.remove(&0x230);
        self.commander_load = crate::commander_restore::apply(
            &mut self.commander,
            source,
            kingdom,
            definition
                .as_ref()
                .map(|d| (&d.definition, d.maximum_experience)),
        )?;
        Ok(())
    }
    ///4E86C3..4E8756, with the actual saved Team handle looked up in the one
    /// identity registry. Missing/stale handles resolve to null as native code.
    pub fn load_kingdom_before_commander(
        &mut self,
        source: &crate::kingdoms::KingdomPayload,
        saved_team: Option<ActorId>,
    ) -> Result<()> {
        if source.team_handle.is_some() {
            self.words.remove(&0x1ec);
            self.identity_references.insert(0x1ec, saved_team);
        }
        if let Some(values) = &source.words_1dc {
            self.vectors
                .get_mut(&0x1dc)
                .ok_or_else(|| Error::Format("Kingdom1DC vector is unconstructed".into()))?
                .replace_counted(values)?;
        }
        if let Some(value) = source.byte_23a {
            self.set_byte(0x23a, value);
        }
        if let Some(recruitment) = &source.recruitment {
            if !recruitment.items.is_empty() {
                return Err(Error::Format("nonempty Kingdom recruitment requires native item construction/definition binding".into()));
            }
            if let Some(word) = recruitment.word_24 {
                self.words.insert(0x220, word);
            }
        }
        Ok(())
    }
    fn set_byte(&mut self, offset: u32, value: u8) {
        if let Some(word) = self.words.get_mut(&(offset & !3)) {
            let shift = (offset & 3) * 8;
            *word = (*word & !(0xff << shift)) | ((value as u32) << shift);
        } else {
            self.bytes.insert(offset, value);
        }
    }
    /// Exact4E85C1..4E86C3 dependency order. References are source-bound by the
    /// adapter; a native pointer value is never synthesized from an IDS string.
    pub fn load_kingdom_prefix(
        &mut self,
        source: &crate::kingdoms::KingdomPayload,
        definitions: &impl crate::restore::RestoreDefinitions,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        for (o, v) in [
            (0x188, source.word_188),
            (0x18c, source.word_18c),
            (0x190, source.word_190),
        ] {
            self.words.insert(o, v);
        }
        self.strings_utf16
            .insert(0x194, source.display_key_utf16.clone());
        self.strings_utf16.insert(
            0x198,
            definitions
                .localize(&source.display_key_utf16)
                .map_err(Error::Format)?,
        );
        let color = if source.color_reference_utf16.is_empty() {
            None
        } else {
            definitions
                .resolve_named_definition("KingdomColor", &source.color_reference_utf16, pc)
                .map_err(Error::Format)?
        };
        if color.is_none() && !source.color_reference_utf16.is_empty() {
            return Err(Error::Format(
                "native KingdomColor lookup rejected nonempty unknown IDS".into(),
            ));
        }
        self.words.remove(&0x1e8);
        self.references.insert(0x1e8, color);
        self.vectors
            .get_mut(&0x1d0)
            .ok_or_else(|| Error::Format("Kingdom relation vector is unconstructed".into()))?
            .replace_counted(&source.words_1d0)?;
        let vector = self
            .vectors
            .get_mut(&0x1c4)
            .ok_or_else(|| Error::Format("Kingdom resource vector is unconstructed".into()))?;
        vector.values.fill(0);
        for source in &source.resources_1c4.values {
            let definition = definitions
                .resolve_named_definition("Resource", &source.reference_utf16, pc)
                .map_err(Error::Format)?;
            if let Some(definition) = definition {
                let index = definition.index.ok_or_else(|| {
                    Error::Format("Resource lookup lacks verified native ordinal".into())
                })?;
                if let Some(value) = vector.values.get_mut(index as usize) {
                    *value = source.word;
                }
            }
        }
        let maximum = definitions
            .stockpile_resource_max_bits(pc)
            .map_err(Error::Format)?;
        for value in &mut vector.values {
            *value = clamp_resource_bits(*value, maximum);
        }
        Ok(())
    }
    /// Original4E6970 after GThing::Load. Translation may fail after the key
    /// has already changed; preserve that partial state and later field order.
    pub fn load_team_fields(
        &mut self,
        source: &crate::kingdoms::TeamRecord,
        mut localize: impl FnMut(&[u16]) -> std::result::Result<Vec<u16>, String>,
    ) -> Result<()> {
        self.words.insert(0x38, source.slot_index);
        self.strings_utf16
            .insert(0x3c, source.display_key_utf16.clone());
        let translated = localize(&source.display_key_utf16).map_err(Error::Format)?;
        self.strings_utf16.insert(0x40, translated);
        self.vectors
            .get_mut(&0x4c)
            .ok_or_else(|| Error::Format("Team4C vector was not constructed".into()))?
            .replace_values(&source.words_4c)?;
        self.vectors
            .get_mut(&0x58)
            .ok_or_else(|| Error::Format("Team58 vector was not constructed".into()))?
            .replace_values(&source.words_58)?;
        if let Some(value) = &source.string_64_utf16 {
            self.strings_utf16.insert(0x64, value.clone());
        }
        Ok(())
    }
    fn empty() -> Self {
        Self {
            words: BTreeMap::new(),
            bytes: BTreeMap::new(),
            strings_utf16: BTreeMap::new(),
            vectors: BTreeMap::new(),
            indexed_words: BTreeMap::new(),
            recruitment_owner: None,
            recruit_items: Default::default(),
            team_members: None,
            actor_lists: None,
            references: BTreeMap::new(),
            identity_references: BTreeMap::new(),
            commander: None,
            commander_load: None,
            tactical: None,
            strategic: None,
            definition_counts: None,
            nation_badge_indices: Vec::new(),
            supply_sources: Vec::new(),
            missing_count_definitions: Vec::new(),
            invalid_nation_indices: Vec::new(),
        }
    }
    /// Full4E67C0 derived portion, after the separately proven Thing load ctor.
    pub fn team() -> Result<Self> {
        let mut s = Self::empty();
        s.team_members = Some(Vec::new());
        s.words.insert(0x68, 0);
        for o in [0x3c, 0x40, 0x64] {
            s.strings_utf16.insert(o, Vec::new());
        }
        for (o, n) in [(0x4c, 32), (0x58, 8)] {
            s.vectors.insert(o, NativeVectorState::empty(n)?);
        }
        Ok(s)
    }
    /// Full4E5370, including4E1CF0/79E130/4E2EA0 derived defaults. Resource
    /// length is the completed global Resource registry count, not capacity8.
    pub fn kingdom(owner: ActorId, resource_count: u32) -> Result<Self> {
        let mut s = Self::empty();
        s.recruitment_owner = Some(owner);
        s.actor_lists = Some(KingdomActorLists {
            by_class: (0..11).map(|_| Vec::new()).collect(),
            flares: Vec::new(),
            city_ownership: Vec::new(),
        });
        for o in [
            0x184, 0x19c, 0x1e8, 0x1ec, 0x1f0, 0x1f4, 0x1f8, 0x20c, 0x224, 0x228, 0x230, 0x234,
            0x244, 0x2f4, 0x324, 0x328, 0x32c, 0x330, 0x338, 0x33c,
            0x340, 0x344, 0x358, 0x364, 0x36c, 0x370,
        ] {
            s.words.insert(o, 0);
        }
        for (o, v) in [
            (0x188, u32::MAX),
            (0x18c, u32::MAX),
            (0x190, 1),
            (0x208, 57),
            (0x220, u32::MAX),
            (0x238, 0x01010001),
            (0x334, 0x42700000),
            (0x360, 10),
        ] {
            s.words.insert(o, v);
        }
        for (o, v) in [
            (0x210, 1),
            (0x22c, 0),
            (0x22d, 0),
            (0x23c, 1),
            (0x23d, 1),
            (0x314, 0),
            (0x368, 1),
        ] {
            s.bytes.insert(o, v);
        }
        for o in [0x194, 0x198, 0x240, 0x248, 0x348] {
            s.strings_utf16.insert(o, Vec::new());
        }
        for (o, n) in [
            (0x1a0, 8),
            (0x1ac, 8),
            (0x1b8, 8),
            (0x1c4, 8),
            (0x1d0, 32),
            (0x1dc, 8),
            (0x214, 10),
            (0x24c, 1),
            (0x258, 8),
            (0x2c4, 10),
            (0x2d0, 3),
            (0x2dc, 5),
            (0x2e8, 20),
            (0x300, 5),
            (0x318, 5),
            (0x34c, 1),
        ] {
            s.vectors.insert(o, NativeVectorState::empty(n)?);
        }
        //45F180 doubles existing8 capacity for9..16, then uses larger counts.
        let capacity = if resource_count > 16 {
            resource_count
        } else if resource_count > 8 {
            16
        } else {
            8
        };
        for o in [0x1c4, 0x258] {
            s.vectors
                .insert(o, NativeVectorState::zeroed(resource_count, capacity)?);
        }
        for (o, n) in [(0x204, 57), (0x35c, 10)] {
            s.indexed_words.insert(o, vec![0; n]);
        }
        Ok(s)
    }
    /// Reconstruct the scalar projection for native fixture comparison without
    /// storing a second mutable copy of vector counts/capacities.
    pub fn scalar_projection(&self) -> BTreeMap<u32, u32> {
        let mut values = self.words.clone();
        // Native process addresses have no portable scalar representation.
        // Empty constructor lists have exact null head/tail witnesses.
        if self.team_members.as_ref().is_some_and(Vec::is_empty) {
            values.insert(0x44, 0);
            values.insert(0x48, 0);
        }
        if let Some(lists) = &self.actor_lists {
            for (index, list) in lists.by_class.iter().enumerate() {
                if list.is_empty() {
                    let offset = 0x26c + 8 * index as u32;
                    values.insert(offset, 0);
                    values.insert(offset + 4, 0);
                }
            }
            if lists.flares.is_empty() {
                values.insert(0x2f8, 0);
                values.insert(0x2fc, 0);
            }
            if lists.city_ownership.is_empty(){
                values.insert(0x30c,0);values.insert(0x310,0);
            }
        }
        for (offset, v) in &self.vectors {
            values.insert(offset + 4, v.values.len() as u32);
            values.insert(offset + 8, v.capacity_hint);
        }
        if let Some(table) = &self.definition_counts {
            values.insert(0x360, table.bucket_count());
            values.insert(0x364, table.len());
        }
        if self.recruitment_owner.is_some() && self.supply_sources.is_empty() {
            values.insert(0x264, 0);
            values.insert(0x268, 0);
        }
        values
    }
}

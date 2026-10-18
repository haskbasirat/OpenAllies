//! Native4C5250 recruitment requirements over the SAME ActorAccess/Bank.
//! Source Element+2C8 owns two head-inserted lists with constructor AND modes.
//! Queries retain actual Properties/Kingdom vectors and never synthesize an
//! empty runtime owner from a source declaration or an inventory census.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent, RuntimeObject, Source, SpecialSource},
    catalog::Catalog,
    definitions::Bank,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_body::credit::Object;
use aa_numeric::Precision;
use aa_production::{
    construction::{StructureMatchWorld, structure_matches_with},
    research::{self, RequirementOutcome, RequirementWorld, Requirements},
};
use aa_rules::{
    actors::ResolvedNode,
    native::{
        TypedValue,
        layout_data::{PropertyRef, RootPosition},
    },
};
#[cfg(test)]
#[path = "stored_requirements_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn production(e: impl std::fmt::Display) -> aa_production::Error {
    aa_production::Error::World(e.to_string())
}

fn source_pc(catalog: &Catalog<'_>, bank: &Bank, pc: Precision) -> Result<()> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_precision() != pc
        || source.source_identities() != catalog.actors.sources.as_slice()
    {
        return Err("4C5250 actual completed Bank source universe/PC differs".into());
    }
    Ok(())
}
fn node<'a>(catalog: &'a Catalog<'_>, id: &str) -> Result<ResolvedNode<'a>> {
    catalog.require_untranslated_actor_identity(id)?;
    let registered = catalog
        .actors
        .find(id)
        .ok_or("4C5250 canonical ActorData absent")?;
    let node = catalog
        .actors
        .resolved_actor(&registered.id)
        .map_err(text)?;
    if node.sources() != registered.template_chain
        || node.sources().first() != Some(&registered.source)
    {
        return Err("4C5250 actual canonical source/template chain differs".into());
    }
    if catalog
        .actors
        .native_category(&registered.id)
        .map_err(text)?
        .native_kind
        != 2
    {
        return Err("4C5250 requirements require actual ActorData".into());
    }
    Ok(node)
}
fn string(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<String> {
    match node.field(name, pc).map_err(text)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!(
            "4C5250 actual source string {name} absent/wrong type"
        )),
    }
}

///464DA1..DBA: actual typed ActorData+1B3, only read when native admission
///failed its property requirements. No source value is treated as current state.
pub(crate) fn hide_if_missing_properties(
    catalog: &Catalog<'_>,
    bank: &Bank,
    id: &str,
    pc: Precision,
) -> Result<bool> {
    source_pc(catalog, bank, pc)?;
    node(catalog, id)?
        .field("hide_if_missing_properties", pc)
        .map_err(text)?
        .ok_or("464DA1 actual hide_if_missing_properties source absent")?
        .boolean()
        .map_err(text)
}

fn requirements(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    id: &str,
    pc: Precision,
) -> Result<Requirements> {
    source_pc(catalog, bank, pc)?;
    let root = node(catalog, id)?;
    let element = root
        .child("ElementComponent")
        .map_err(text)?
        .ok_or("4C5250 actual Element ActorData+2C8 allocation is absent")?;
    let mask = catalog.canonical_component_mask(id, pc)?;
    if mask & (1 << Slot::Element.index()) == 0 {
        return Err("4C5250 actual Element source factory mask differs".into());
    }
    //The source loader schedules structures as a deferred link and loads
    //properties via4C5720. Both completed lists use head insertion.
    let structures = aa_rules::abilities::required_property_tokens(&string(
        &element,
        "required_structures",
        pc,
    )?)
    .map_err(text)?;
    let properties = aa_rules::abilities::required_property_tokens(&string(
        &element,
        "required_properties",
        pc,
    )?)
    .map_err(text)?;
    let mut out = Requirements::default(); //actual4C51E0 writes both mode bytes0.
    for authored in structures {
        let target = catalog
            .actors
            .find(&authored)
            .ok_or_else(|| format!("4C54B0 unknown required Structure {authored}"))?;
        if catalog.canonical_component_mask(&target.id, pc)? & (1 << Slot::Structure.index()) == 0 {
            return Err(format!(
                "4C54B0 required actor {} is not a Structure",
                target.id
            ));
        }
        let token = bank.intern("ActorDefinition", &target.id)?;
        if out.actors.contains(&token) {
            return Err("4C54B0 duplicate requires actual source diagnostic continuation".into());
        }
        out.actors.insert(0, token);
    }
    if !properties.is_empty() {
        let registered = catalog
            .actors
            .find(id)
            .ok_or("4C5720 canonical source root absent")?;
        let file = catalog
            .actors
            .sources
            .iter()
            .position(|s| s.path.eq_ignore_ascii_case(&registered.source.path))
            .ok_or("4C5720 actual source file ordinal absent")?;
        let at = RootPosition {
            file,
            offset: registered.source.span.start,
        };
        let layout = catalog.layout_data(pc)?;
        actors.property_allocations.bind_in_bank(catalog, bank)?;
        for authored in properties {
            let reference = layout
                .find_property(&authored, Some(at))
                .ok_or_else(|| format!("4C5720 unknown required Property {authored}"))?;
            let token = actors
                .property_allocations
                .definition_for_reference(reference)?
                .0;
            if out.properties.contains(&token) {
                return Err(
                    "4C5720 duplicate requires actual source diagnostic continuation".into(),
                );
            }
            out.properties.insert(0, token);
        }
    }
    Ok(out)
}

///581F70 passes the actual recruiter as actor, its virtual108 Kingdom, and
///4CED40's real Settlement parent. The reused4C5250 kernel preserves property
///checks before structure checks even when the former already failed.
pub(crate) fn evaluate(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    bank: &mut Bank,
    recruiter: ActorId,
    target: &str,
    pc: Precision,
) -> Result<RequirementOutcome> {
    let requirements = requirements(catalog, actors, bank, target, pc)?;
    let mut world = Bound {
        catalog,
        actors,
        bank,
        pc,
    };
    world.object(recruiter)?;
    let settlement = world.settlement_parent(recruiter)?;
    let kingdom = world.kingdom_of(recruiter)?;
    research::evaluate_requirements(
        &requirements,
        kingdom.map(ActorId::raw),
        settlement.map(ActorId::raw),
        Some(recruiter.raw()),
        &mut world,
    )
    .map_err(text)
}

struct Bound<'a, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    actors: &'a mut ActorAccess<'store>,
    bank: &'a mut Bank,
    pc: Precision,
}
impl Bound<'_, '_, '_> {
    fn object(&self, id: ActorId) -> Result<&RuntimeObject> {
        let key = self
            .actors
            .key(id)
            .ok_or("4C5250 current Actor generation absent")?;
        let object = self.actors.object(key).map_err(text)?;
        let source = crate::live_object_init::Source::bind_in_bank(
            self.catalog,
            self.bank,
            &object.definition,
        )
        .map_err(text)?;
        if source.source_precision() != self.pc
            || object.actor.is_none()
            || object.object.is_none()
            || !matches!(object.definition.native_class, 3 | 4)
            || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
        {
            return Err("4C5250 actual Actor source/layout/PC differs".into());
        }
        Ok(object)
    }
    fn kingdom_of(&self, id: ActorId) -> Result<Option<ActorId>> {
        let word = *self
            .object(id)?
            .actor
            .as_ref()
            .unwrap()
            .words
            .get(&0xe8)
            .ok_or("581F70 actual ActorE8 uninitialized")?;
        if word == 0 {
            return Ok(None);
        }
        let kingdom = ActorId::from_raw(word);
        self.kingdom(kingdom)?;
        Ok(Some(kingdom))
    }
    fn kingdom(&self, id: ActorId) -> Result<&aa_mapworld::special_state::SpecialShell> {
        if self.object(id)?.definition.native_class != 4 {
            return Err("4C5250 virtual108 owner is not a Kingdom".into());
        }
        let key = self
            .actors
            .key(id)
            .ok_or("4C5250 Kingdom generation absent")?;
        let Source::Special(k) = &self.actors.entity(key).map_err(text)?.source else {
            return Err("4C5250 same Kingdom derived allocation absent".into());
        };
        if !matches!(k.source, SpecialSource::Kingdom(_)) {
            return Err("4C5250 special source is not Kingdom".into());
        }
        let derived = k
            .derived
            .as_ref()
            .ok_or("4C5250 actual Kingdom fields absent")?;
        if derived.recruitment_owner != Some(id) {
            return Err("4C5250 Kingdom constructor owner differs".into());
        }
        Ok(derived)
    }
    fn settlement_parent(&self, id: ActorId) -> Result<Option<ActorId>> {
        let object = self.object(id)?;
        let components = object.components().map_err(text)?;
        if components.get(Slot::Settlement).is_some() {
            return Ok(Some(id));
        }
        if let Some(component) = components.get(Slot::Structure) {
            let RuntimeComponent::Structure(s) = component else {
                return Err("4CED60 actual Structure payload differs".into());
            };
            if s.header.owner != id {
                return Err("4CED60 actual Structure owner differs".into());
            }
            if s.object_handle_10 != 0 {
                let parent = ActorId::from_raw(s.object_handle_10);
                self.object(parent)?;
                return Ok(Some(parent));
            }
        }
        if components.get(Slot::Wall).is_some() {
            return Err("4CED60 positive Wall58CB30 actual parent owner required".into());
        }
        Ok(None)
    }
    fn property_list(
        &self,
        owner: ActorId,
    ) -> Result<Option<&aa_body::properties::PropertiesState>> {
        match self
            .object(owner)?
            .components()
            .map_err(text)?
            .get(Slot::Properties)
        {
            None => Ok(None),
            Some(RuntimeComponent::Properties(p)) if p.owner.0 == owner.raw() => Ok(Some(p)),
            _ => Err("581280 actual Properties allocation/owner differs".into()),
        }
    }
    fn contains(&self, list: &[Object], definition: u32) -> Result<bool> {
        if list.is_empty() {
            return Ok(false);
        }
        self.actors
            .property_allocations
            .require_in_bank(self.catalog, self.bank)?;
        aa_body::property_api::find_instance(list, Object(definition), None, None, |handle| {
            let instance = self
                .actors
                .property_allocations
                .instances()
                .get(&handle)
                .ok_or_else(|| {
                    aa_body::Error::Callback("5032D0 actual instance allocation absent".into())
                })?;
            //The actual call's source argument is NULL. It never resolves+14.
            Ok((instance.definition, None))
        })
        .map(|v| v.is_some())
        .map_err(text)
    }
    fn structure(&self, id: ActorId) -> Result<()> {
        match self
            .object(id)?
            .components()
            .map_err(text)?
            .get(Slot::Structure)
        {
            Some(RuntimeComponent::Structure(s)) if s.header.owner == id => Ok(()),
            _ => Err("4E1480 required actual Structure allocation/owner absent".into()),
        }
    }
}
impl RequirementWorld for Bound<'_, '_, '_> {
    fn properties_of(&mut self, owner: u32) -> aa_production::Result<Option<u32>> {
        Ok(self
            .property_list(ActorId::from_raw(owner))
            .map_err(production)?
            .map(|_| owner))
    }
    fn properties_contain(&mut self, owner: u32, definition: u32) -> aa_production::Result<bool> {
        let p = self
            .property_list(ActorId::from_raw(owner))
            .map_err(production)?
            .ok_or_else(|| production("581280 current Properties disappeared"))?;
        self.contains(&p.effects, definition).map_err(production)
    }
    fn is_technology(&mut self, definition: u32) -> aa_production::Result<bool> {
        let facts = self
            .actors
            .property_allocations
            .facts(Object(definition))
            .map_err(production)?;
        let PropertyRef::Registered(index) = facts.reference() else {
            return Err(production(
                "4C5250 required Property is not a registered source definition",
            ));
        };
        let data = self.catalog.layout_data(self.pc).map_err(production)?;
        let registered = data.properties.get(index as usize).ok_or_else(|| {
            production("4C5250 actual registered Property descriptor unavailable")
        })?;
        if registered.ordinal != index
            || registered.reloads != 0
            || !registered.ids.eq_ignore_ascii_case(facts.id())
        {
            return Err(production(
                "4C5250 actual registered Property class/identity differs",
            ));
        }
        //The RequirementWorld method's historical name is misleading: native
        //reads vf18 (registered/shared PropertyData type), NOT byte5C
        //is_technology. The registered class's actual leaf is constant TRUE.
        Ok(true)
    }
    fn kingdom_knows(&mut self, owner: u32, definition: u32) -> aa_production::Result<bool> {
        let list = &self
            .kingdom(ActorId::from_raw(owner))
            .map_err(production)?
            .vectors
            .get(&0x318)
            .ok_or_else(|| production("4E0FB0 actual Kingdom318 instance vector absent"))?
            .values;
        if list.is_empty() {
            return Ok(false);
        }
        self.actors
            .property_allocations
            .require_in_bank(self.catalog, self.bank)
            .map_err(production)?;
        //No source filtering, no disabled/kind predicate: native5032D0 matches+C.
        for &handle in list {
            let instance = self
                .actors
                .property_allocations
                .instances()
                .get(&Object(handle))
                .ok_or_else(|| production("4E0FB0 actual Kingdom instance allocation absent"))?;
            if instance.definition == Object(definition) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn settlement_component(&mut self, owner: u32) -> aa_production::Result<Option<u32>> {
        if self
            .object(ActorId::from_raw(owner))
            .map_err(production)?
            .components()
            .map_err(production)?
            .get(Slot::Settlement)
            .is_some()
        {
            return Err(production(
                "4C5250 positive actual Settlement payload owner585500 required",
            ));
        }
        Ok(None)
    }
    fn settlement_has_actor(&mut self, _: u32, _: u32) -> aa_production::Result<bool> {
        Err(production(
            "585500 actual retained Settlement component owner required",
        ))
    }
    fn kingdom_has_actor(&mut self, owner: u32, definition: u32) -> aa_production::Result<bool> {
        let kingdom = ActorId::from_raw(owner);
        let mut index = 0;
        loop {
            let values = &self
                .kingdom(kingdom)
                .map_err(production)?
                .vectors
                .get(&0x2c4)
                .ok_or_else(|| production("4E1480 actual Kingdom2C4 structure vector absent"))?
                .values;
            let Some(&raw) = values.get(index) else {
                return Ok(false);
            };
            let actor = ActorId::from_raw(raw);
            self.structure(actor).map_err(production)?;
            if structure_matches_with(
                &mut Structure { world: self, actor },
                definition,
                true,
                false,
            )? {
                return Ok(true);
            }
            index += 1;
        }
    }
    fn technology_allowed(&mut self, _: u32, _: u32) -> aa_production::Result<bool> {
        Err(production("4E1890 is outside direct4C5250"))
    }
    fn describe_missing(&mut self, _: bool, _: &[u32]) -> aa_production::Result<()> {
        Err(production(
            "4C5560 diagnostic destination not passed by direct4C5250",
        ))
    }
    fn missing_requirements_message(
        &mut self,
        _: &research::ResearchItem,
    ) -> aa_production::Result<()> {
        Err(production(
            "405D60 diagnostic destination not passed by direct4C5250",
        ))
    }
}
struct Structure<'a, 'b, 'store, 'source> {
    world: &'a Bound<'b, 'store, 'source>,
    actor: ActorId,
}
impl StructureMatchWorld for Structure<'_, '_, '_, '_> {
    fn state_25(&mut self) -> aa_production::Result<bool> {
        Ok(self
            .world
            .object(self.actor)
            .map_err(production)?
            .components()
            .map_err(production)?
            .state
            .low
            & (1 << 25)
            != 0)
    }
    fn transient(&mut self) -> aa_production::Result<bool> {
        let state = &self
            .world
            .object(self.actor)
            .map_err(production)?
            .components()
            .map_err(production)?
            .state;
        Ok(state.low & (1 << 26) != 0 || state.high & (1 << 4) != 0)
    }
    fn definition(&mut self) -> aa_production::Result<u32> {
        let id = &self
            .world
            .object(self.actor)
            .map_err(production)?
            .definition
            .proof
            .id;
        self.world
            .bank
            .token("ActorDefinition", id)
            .ok_or_else(|| production("5890C0 actual ActorDefinition token absent"))
    }
    fn contains(&mut self, offset: u32, _: u32) -> aa_production::Result<bool> {
        if !matches!(offset, 0x4c0 | 0x4d0) {
            return Err(production("5890C0 unsupported source list"));
        }
        //The existing strict whole-input receipt proves no Upgrade callbacks
        //can populate either outgoing4C0 or predecessor4D0 in these sources.
        let census = self
            .world
            .catalog
            .actors
            .construction_upgrade_census()
            .map_err(production)?;
        if census.sources != self.world.catalog.actors.sources
            || !census.explicit_instances.is_empty()
        {
            return Err(production(
                "5890C0 nonempty/foreign Upgrade source graph requires actual linked lists",
            ));
        }
        Ok(false)
    }
    fn constructing(&mut self) -> aa_production::Result<bool> {
        Ok(self
            .world
            .object(self.actor)
            .map_err(production)?
            .components()
            .map_err(production)?
            .state
            .low
            & (1 << 21)
            != 0)
    }
}

//! Native4E3AD0/5036D0 over the actual stored Kingdom+318 vector and the same
//! pre-staging property allocation owner. No mirrored PropertyRecord vector.
use crate::{
    catalog::Catalog,
    live_object_init::Record,
    load_property_instances::{self as property, State},
    restriction_groups::LoadedRestrictionGroups,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_body::credit::Object;
use aa_mapworld::{
    restore::{RestoredComponent, RestoredRecord},
    special_state::SpecialShell,
};
use aa_numeric::Precision;
use aa_rules::native::layout_data::PropertyRef;
type Result<T> = std::result::Result<T, String>;

///Native404F60 denial and42D4B0 event5 must be bound to their real owners.
///The latter includes the reached Data+10 name and GameEventManager service;
///a source metadata label does not stand in for that runtime allocation.
pub trait Notifications {
    fn denied(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        kingdom: ActorId,
        definition: Object,
    ) -> Result<()>;
    fn gained(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        kingdom: ActorId,
        definition: Object,
    ) -> Result<()>;
}
pub struct RequiredNotifications;
impl Notifications for RequiredNotifications {
    fn denied(&mut self, _: &Catalog<'_>, _: &mut Record<'_>, _: ActorId, _: Object) -> Result<()> {
        Err("4E3B18 actual Kingdom name/404F60 denial diagnostic owner is required".into())
    }
    fn gained(&mut self, _: &Catalog<'_>, _: &mut Record<'_>, _: ActorId, _: Object) -> Result<()> {
        Err("4E3BF5 actual PropertyData+10 name/42D4B0 GameEventManager owner is required".into())
    }
}
pub struct Context<'a, E> {
    pub instances: &'a mut State,
    pub effects: &'a mut E,
    pub restrictions: Option<&'a LoadedRestrictionGroups>,
    pub current_time: &'a mut dyn FnMut() -> Result<u32>,
    pub notifications: &'a mut dyn Notifications,
}
fn kingdom<'a>(record: &'a Record<'_>, id: ActorId) -> Result<&'a SpecialShell> {
    if id == record.owner {
        return Err(
            "4E3AD0 stored-Kingdom binding requires its actual disjoint derived allocation".into(),
        );
    }
    let entry = record
        .registry
        .get(id)
        .ok_or("4E3AD0 current Kingdom generation is absent")?;
    let RestoredRecord::Kingdom(k) = &entry.value else {
        return Err("4E3AD0 current owner is not a stored Kingdom".into());
    };
    if k.source.serialized_id != id.raw() {
        return Err("4E3AD0 serialized Kingdom identity differs".into());
    }
    let runtime = k
        .runtime
        .as_ref()
        .ok_or("4E3AD0 actual Kingdom allocation is unavailable")?;
    if runtime.definition.native_class != 4 || runtime.base.actor.is_none() {
        return Err("4E3AD0 actual Kingdom Actor layout differs".into());
    }
    let derived = runtime
        .derived
        .as_ref()
        .ok_or("4E3AD0 actual derived Kingdom allocation is absent")?;
    if derived.recruitment_owner != Some(id) {
        return Err("4E3AD0 actual Kingdom constructor identity differs".into());
    }
    Ok(derived)
}
fn kingdom_mut<'a>(record: &'a mut Record<'_>, id: ActorId) -> Result<&'a mut SpecialShell> {
    kingdom(record, id)?;
    let Some(RestoredRecord::Kingdom(k)) = record.registry.value_mut(id) else {
        return Err("4E3AD0 current Kingdom allocation changed".into());
    };
    k.runtime
        .as_mut()
        .and_then(|v| v.derived.as_mut())
        .ok_or("4E3AD0 actual derived Kingdom changed".into())
}
fn source(catalog: &Catalog<'_>, definition: Object) -> Result<(PropertyRef, bool)> {
    catalog.require_source_owner()?;
    let bank = catalog
        .definition_bank
        .try_borrow()
        .map_err(|_| "4E3AD0 source Bank is borrowed")?;
    let pc = bank.combat_source()?.source_precision();
    if bank.combat_source()?.source_identities() != catalog.actors.sources.as_slice() {
        return Err("4E3AD0 Property source universe differs".into());
    }
    let entry = bank.entry(definition.0)?;
    if entry.key.domain != "property" {
        return Err("4E3AD0 definition belongs to another native manager".into());
    }
    let canonical = entry.key.name.clone();
    drop(bank);
    let data = catalog.layout_data(pc)?;
    let matches = data
        .properties
        .iter()
        .filter(|v| v.ids.eq_ignore_ascii_case(&canonical))
        .collect::<Vec<_>>();
    let [p] = matches.as_slice() else {
        return Err("4E3AD0 canonical registered PropertyData is absent/ambiguous".into());
    };
    if p.reloads != 0 {
        return Err("4E3AD0 reloaded PropertyData field owner is required".into());
    }
    //Registered PropertyData uses the real shared-property virtual18 true;
    //its independently loaded+5C byte is is_technology, not that type test.
    Ok((
        PropertyRef::Registered(p.ordinal),
        p.is_technology
            .ok_or("4E3AD0 PropertyData+5C has not been loaded")?,
    ))
}
fn key(value: &[u16]) -> Result<String> {
    let end = value.iter().position(|v| *v == 0).unwrap_or(value.len());
    if value[..end].iter().any(|v| *v > 0x7f) {
        return Err("4E1890 profile key requires the proven ASCII C-locale domain".into());
    }
    Ok(value[..end].iter().map(|v| char::from(*v as u8)).collect())
}
fn allowed(
    catalog: &Catalog<'_>,
    record: &Record<'_>,
    id: ActorId,
    definition: Object,
    loaded: Option<&LoadedRestrictionGroups>,
) -> Result<bool> {
    let loaded = loaded.ok_or("4E1890 actual loaded restriction profile owner is required")?;
    loaded.require_catalog(catalog)?;
    let bank = catalog
        .definition_bank
        .try_borrow()
        .map_err(|_| "4E1890 Property Bank is borrowed")?;
    let name = bank.entry(definition.0)?.key.name.clone();
    drop(bank);
    let k = kingdom(record, id)?;
    let team = match k.identity_references.get(&0x1ec) {
        Some(v) => *v,
        None if k.words.get(&0x1ec) == Some(&0) => None,
        _ => return Err("4E1890 actual Kingdom+1EC Team identity is absent".into()),
    };
    if let Some(team) = team {
        let entry = record
            .registry
            .get(team)
            .ok_or("4E1890 current Team generation is absent")?;
        let RestoredRecord::Team(t) = &entry.value else {
            return Err("4E1890 current Team record has another native class".into());
        };
        let special = t
            .runtime
            .as_ref()
            .and_then(|v| v.derived.as_ref())
            .ok_or("4E1890 actual Team allocation is absent")?;
        let profile = key(special
            .strings_utf16
            .get(&0x64)
            .ok_or("4E1890 actual Team+64 profile name is absent")?)?;
        if !aa_economy::restrictions::technology_allowed(
            loaded.profiles().find(&profile),
            None,
            &name,
        ) {
            return Ok(false);
        }
    }
    let profile = key(kingdom(record, id)?
        .strings_utf16
        .get(&0x348)
        .ok_or("4E1890 actual Kingdom+348 profile name is absent")?)?;
    Ok(aa_economy::restrictions::technology_allowed(
        None,
        loaded.profiles().find(&profile),
        &name,
    ))
}
fn actors<'a>(record: &'a Record<'_>, id: ActorId) -> Result<&'a [ActorId]> {
    kingdom(record, id)?
        .actor_lists
        .as_ref()
        .ok_or("4E3AD0 actual Kingdom class-list owner is absent")?
        .by_class
        .get(3)
        .map(Vec::as_slice)
        .ok_or("4E3AD0 actual Kingdom+284 Actor class list is absent".into())
}
fn apply_current<E: property::Effects>(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    definition: Object,
    kind: u32,
    origin: Option<ActorId>,
    context: &mut Context<'_, E>,
    pc: Precision,
) -> Result<()> {
    if definition.0 == 0 {
        return Ok(());
    } //Native4CEF10 null property return.
    let a = record
        .shell
        .actor
        .as_ref()
        .ok_or("4CEF10 actual Actor storage is required")?;
    match a.components.get(ComponentSlot::Properties) {
        None => return Ok(()),
        Some(RestoredComponent::Properties(p)) if p.owner.0 == record.owner.raw() => {}
        _ => return Err("4CEF10 actual Properties payload/owner differs".into()),
    }
    let reference = context
        .instances
        .facts(definition)?
        .reference()
        .handle()
        .map_err(|e| e.to_string())?;
    if !([8, 10].contains(&kind) && a.components.present(ComponentSlot::Denizen))
        && !property::applicable(catalog, record, context.instances, reference)?
    {
        return Ok(());
    }
    property::add(
        catalog,
        record,
        context.instances,
        reference,
        kind,
        origin,
        || (context.current_time)(),
        pc,
        context.effects,
    )
}
fn apply<E: property::Effects>(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    target: ActorId,
    definition: Object,
    kind: u32,
    origin: Option<ActorId>,
    context: &mut Context<'_, E>,
    pc: Precision,
) -> Result<()> {
    if target == record.owner {
        return apply_current(catalog, record, definition, kind, origin, context, pc);
    }
    let value = record
        .registry
        .value_mut(target)
        .ok_or("4CEF10 current class-list Actor generation is absent")?;
    if !matches!(value, RestoredRecord::Actor(_)) {
        return Err("4CEF10 current class-list target is not stored Actor layout".into());
    }
    let mut held = std::mem::replace(value, RestoredRecord::Loading);
    let result = if let RestoredRecord::Actor(a) = &mut held {
        let mut target_record = Record {
            owner: target,
            shell: &mut a.shell,
            definition: &a.definition,
            registry: &mut *record.registry,
        };
        apply_current(
            catalog,
            &mut target_record,
            definition,
            kind,
            origin,
            context,
            pc,
        )
    } else {
        unreachable!("checked current Actor record")
    };
    let current = record
        .registry
        .value_mut(target)
        .ok_or("4CEF10 held target identity disappeared")?;
    if !matches!(current, RestoredRecord::Loading) {
        return Err("4CEF10 held target slot was replaced during callback".into());
    }
    *current = held;
    result
}

///Stored-Kingdom domain reached by4E6180 after the same held Actor is registered.
///Canonical definition is a SAME Bank Property token (not PropertyRef ordinal).
///A false result is a denied/duplicate grant; allocation/effect failures retain
///the shared constructor/instance/vector and actor mutation prefix.
#[allow(clippy::too_many_arguments)]
pub fn add<E: property::Effects>(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    kingdom_id: ActorId,
    definition: u32,
    kind: u32,
    origin: Option<ActorId>,
    notify: bool,
    context: &mut Context<'_, E>,
    pc: Precision,
) -> Result<bool> {
    let definition = Object(definition);
    let (reference, technology) = source(catalog, definition)?;
    if technology
        && !allowed(
            catalog,
            record,
            kingdom_id,
            definition,
            context.restrictions,
        )?
    {
        context
            .notifications
            .denied(catalog, record, kingdom_id, definition)?;
        return Ok(false);
    }
    context.instances.bind_registered_sources(catalog)?;
    if context.instances.definition_for_reference(reference)? != definition {
        return Err("4E3AD0 Property reference and actual Bank token differ".into());
    }
    let instance =
        property::construct_instance(context.instances, definition, kind, origin, || {
            (context.current_time)()
        })?;
    let handle = instance.handle;
    context.instances.instances_mut().insert(handle, instance);
    let mut duplicate = false;
    let mut index = 0;
    loop {
        let list = kingdom(record, kingdom_id)?
            .vectors
            .get(&0x318)
            .ok_or("5036D0 actual Kingdom+318 vector is absent")?;
        let Some(&raw) = list.values.get(index) else {
            break;
        };
        let existing = context
            .instances
            .instances()
            .get(&Object(raw))
            .ok_or("5032D0 current Kingdom property instance allocation is absent")?;
        if existing.definition == definition {
            duplicate = true;
            break;
        }
        index += 1;
    }
    kingdom_mut(record, kingdom_id)?
        .vectors
        .get_mut(&0x318)
        .ok_or("5036D0 actual Kingdom property vector changed")?
        .push(handle.0)
        .map_err(|e| e.to_string())?;
    if duplicate {
        context
            .instances
            .instances_mut()
            .get_mut(&handle)
            .ok_or("5036D0 new allocation disappeared")?
            .disabled = true;
        return Ok(false);
    }
    //The existing class-list owner models native order without node handles.
    //Retain an immutable traversal receipt, never a mutable authoritative copy.
    //A callback mutation requires the real node owner and stops after its writes
    //before the native next-link read; do not continue with snapshot semantics.
    let mut receipt = Vec::new();
    receipt
        .try_reserve(actors(record, kingdom_id)?.len())
        .map_err(|e| e.to_string())?;
    receipt.extend_from_slice(actors(record, kingdom_id)?);
    for target in &receipt {
        let instance = context
            .instances
            .instances()
            .get(&handle)
            .ok_or("4E3BC0 current Kingdom property allocation is absent")?;
        let (definition, kind, origin) = (
            instance.definition,
            instance.kind,
            ActorId::from_raw(instance.source_id),
        );
        let origin = record.registry.get(origin).map(|e| e.id);
        apply(
            catalog, record, *target, definition, kind, origin, context, pc,
        )?;
        if actors(record, kingdom_id)? != receipt.as_slice() {
            return Err("4E3BE1 mutated class-list traversal requires its actual native node identity owner".into());
        }
    }
    if notify {
        let definition = context
            .instances
            .instances()
            .get(&handle)
            .ok_or("4E3BF5 current Kingdom property allocation is absent")?
            .definition;
        context
            .notifications
            .gained(catalog, record, kingdom_id, definition)?;
    }
    Ok(true)
}

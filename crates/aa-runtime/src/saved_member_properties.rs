//! Native5784C0 property propagation while an Organization is held by Load.
//!
//! The owner Properties component is captured once, but its current signed
//! count and each slot are read in native order. Member/instance/source reads
//! remain lazy. An unbound nonempty saved instance arena is a required error,
//! never an empty owner-property list or a successful property application.
use crate::catalog::Catalog;
use aa_actors::{
    ActorId, components::ComponentSlot, loading::ActorShell, registry::IdentityRegistry,
};
use aa_body::credit::Object;
use aa_mapworld::restore::{OrganizationAttachContext, RestoredComponent, RestoredRecord};

#[cfg(test)]
#[path = "saved_member_properties_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;

/// Read services for the unchanged5784C0 coordinator. There is deliberately no
/// snapshot `Vec<OwnerProperty>`: an apply may change the captured list/count.
pub(crate) trait LookupWorld {
    type Properties: Copy;
    fn properties(&self, actor: ActorId) -> Result<Option<Self::Properties>>;
    fn count(&self, properties: Self::Properties) -> Result<i32>;
    fn slot(&self, properties: Self::Properties, index: i32) -> Result<Object>;
    fn instance_definition(&self, instance: Object) -> Result<Object>;
    fn instance_source(&self, instance: Object) -> Result<ActorId>;
}

pub(crate) trait World: LookupWorld {
    fn owner(&self) -> Result<ActorId>;
    fn pass_down_members(&self, definition: Object) -> Result<bool>;
    fn has_experience(&self, actor: ActorId) -> Result<bool>;
    fn instance_kind(&self, instance: Object) -> Result<u32>;
    fn instance_disabled(&self, instance: Object) -> Result<bool>;
    fn apply(&mut self, actor: ActorId, property: Object, kind: u32, source: ActorId)
    -> Result<()>;
}

pub(crate) fn lookup(
    world: &impl LookupWorld,
    actor: ActorId,
    property: Object,
    source: ActorId,
) -> Result<Option<Object>> {
    let Some(properties) = world.properties(actor)? else {
        return Ok(None);
    };
    //5032D6 captures the count and original vector base. Registry resolution
    //is a pure read; there is no callback which may replace that allocation.
    let count = world.count(properties)?;
    for index in 0..count {
        let instance = world.slot(properties, index)?;
        if world.instance_definition(instance)? == property
            && (source.is_null() || world.instance_source(instance)? == source)
        {
            return world.slot(properties, index).map(Some);
        }
    }
    Ok(None)
}

pub(crate) fn propagate(world: &mut impl World, member: ActorId) -> Result<()> {
    let owner = world.owner()?;
    let Some(properties) = world.properties(owner)? else {
        return Ok(());
    };
    let mut index = 0i32;
    if world.count(properties)? <= 0 {
        return Ok(());
    }
    loop {
        //5784E2/E5 fetches this live slot even for a null member. It does not
        //read the instance itself until after the member+78 presence gate.
        let instance = world.slot(properties, index)?;
        if !member.is_null() && world.properties(member)?.is_some() {
            let definition = world.instance_definition(instance)?;
            if world.pass_down_members(definition)?
                && !(world.has_experience(member)? && world.instance_kind(instance)? == 7)
                && !world.instance_disabled(instance)?
            {
                //578512 rereads org+4;578515 rereads instance+8 even when the
                //Experience gate has already read it. Property ECX is retained.
                let source = world.owner()?;
                let kind = world.instance_kind(instance)?;
                world.apply(member, definition, kind, source)?;
            }
        }
        let count = world.count(properties)?;
        index = index.wrapping_add(1);
        if index >= count {
            return Ok(());
        }
    }
}

/// Native5784C0(member) over the same actual owner, Organization and registry
/// used by the saved Organization loader. No preflight resolves a skipped
/// member, property instance, definition or application service.
pub fn member_added(
    catalog: &Catalog<'_>,
    context: OrganizationAttachContext<'_>,
    member: ActorId,
) -> Result<()> {
    propagate(&mut Borrowed { catalog, context }, member)
}

/// Native4CEF80/581280/5032D0 presence query on the held owner. Null source
/// means no source filter; disabled records are included by the native query.
/// Empty/absent actual Properties does not demand definitions or instances.
pub fn find(
    _catalog: &Catalog<'_>,
    context: OrganizationAttachContext<'_>,
    property: u32,
    source: ActorId,
) -> Result<bool> {
    find_actual(
        LookupContext {
            owner: context.owner,
            owner_actor: context.owner_actor,
            registry: context.registry,
        },
        property,
        source,
    )
}

/// Read-only native4CEF80 input. It does not require or fabricate an
/// Organization, mutable world, precision setting or definition catalog.
pub struct LookupContext<'a> {
    pub owner: ActorId,
    pub owner_actor: &'a ActorShell<RestoredComponent>,
    pub registry: &'a IdentityRegistry<RestoredRecord>,
}

pub fn find_actual(context: LookupContext<'_>, property: u32, source: ActorId) -> Result<bool> {
    lookup(&context, context.owner, Object(property), source).map(|value| value.is_some())
}

struct Borrowed<'a, 'world, 'source> {
    catalog: &'a Catalog<'source>,
    context: OrganizationAttachContext<'world>,
}

impl Borrowed<'_, '_, '_> {
    fn lookup_context(&self) -> LookupContext<'_> {
        LookupContext {
            owner: self.context.owner,
            owner_actor: self.context.owner_actor,
            registry: self.context.registry,
        }
    }
    fn shell(&self, actor: ActorId) -> Result<&ActorShell<RestoredComponent>> {
        self.lookup_context().shell(actor)
    }
    fn properties_state(
        &self,
        actor: ActorId,
    ) -> Result<Option<&aa_body::properties::PropertiesState>> {
        self.lookup_context().properties_state(actor)
    }
}

impl<'a> LookupContext<'a> {
    fn shell(&self, actor: ActorId) -> Result<&'a ActorShell<RestoredComponent>> {
        if actor == self.owner {
            return Ok(self.owner_actor);
        }
        self.registry
            .get(actor)
            .and_then(|entry| match &entry.value {
                RestoredRecord::Actor(entity) => entity.shell.actor.as_ref(),
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "5784C0 actor {:#010x} is not a current restored Actor",
                    actor.raw()
                )
            })
    }
    fn properties_state(
        &self,
        actor: ActorId,
    ) -> Result<Option<&'a aa_body::properties::PropertiesState>> {
        match self.shell(actor)?.components.get(ComponentSlot::Properties) {
            None => Ok(None),
            Some(RestoredComponent::Properties(state)) if state.owner == Object(actor.raw()) => {
                Ok(Some(state))
            }
            Some(RestoredComponent::Properties(_)) => {
                Err("5784C0 Properties belongs to a different Actor".into())
            }
            Some(_) => Err("5784C0 Properties slot payload mismatch".into()),
        }
    }
    fn captured_properties(
        &self,
        handle: (ActorId, usize),
    ) -> Result<&'a aa_body::properties::PropertiesState> {
        let state = self
            .properties_state(handle.0)?
            .ok_or("5784C0 captured Properties was detached")?;
        if state as *const _ as usize != handle.1 {
            return Err("5784C0 captured Properties ownership was replaced".into());
        }
        Ok(state)
    }
}

impl LookupWorld for LookupContext<'_> {
    //The address only verifies stable owned allocation identity. It is never
    //dereferenced or serialized as a native pointer.
    type Properties = (ActorId, usize);
    fn properties(&self, actor: ActorId) -> Result<Option<Self::Properties>> {
        Ok(self
            .properties_state(actor)?
            .map(|state| (actor, state as *const _ as usize)))
    }
    fn count(&self, properties: Self::Properties) -> Result<i32> {
        i32::try_from(self.captured_properties(properties)?.effects.len())
            .map_err(|_| "5784C0 Properties count exceeds native signed count".into())
    }
    fn slot(&self, properties: Self::Properties, index: i32) -> Result<Object> {
        let state = self.captured_properties(properties)?;
        usize::try_from(index)
            .ok()
            .and_then(|i| state.effects.get(i).copied())
            .ok_or_else(|| format!("5784C0 live Properties slot {index} is unavailable"))
    }
    fn instance_definition(&self, instance: Object) -> Result<Object> {
        Err(format!(
            "5784C0 instance {:#010x}+C requires the actual saved property-instance arena",
            instance.0
        ))
    }
    fn instance_source(&self, instance: Object) -> Result<ActorId> {
        Err(format!(
            "5032D0 instance {:#010x}+14 requires the actual saved property-instance arena",
            instance.0
        ))
    }
}

impl LookupWorld for Borrowed<'_, '_, '_> {
    type Properties = (ActorId, usize);
    fn properties(&self, actor: ActorId) -> Result<Option<Self::Properties>> {
        Ok(self
            .properties_state(actor)?
            .map(|state| (actor, state as *const _ as usize)))
    }
    fn count(&self, properties: Self::Properties) -> Result<i32> {
        self.lookup_context().count(properties)
    }
    fn slot(&self, properties: Self::Properties, index: i32) -> Result<Object> {
        self.lookup_context().slot(properties, index)
    }
    fn instance_definition(&self, instance: Object) -> Result<Object> {
        self.lookup_context().instance_definition(instance)
    }
    fn instance_source(&self, instance: Object) -> Result<ActorId> {
        self.lookup_context().instance_source(instance)
    }
}

impl World for Borrowed<'_, '_, '_> {
    fn owner(&self) -> Result<ActorId> {
        Ok(self.context.organization.header.owner)
    }
    fn pass_down_members(&self, definition: Object) -> Result<bool> {
        self.catalog.require_source_owner()?;
        Err(format!(
            "5784C0 PropertyData {:#010x}+42 source is unavailable",
            definition.0
        ))
    }
    fn has_experience(&self, actor: ActorId) -> Result<bool> {
        match self.shell(actor)?.components.get(ComponentSlot::Experience) {
            None => Ok(false),
            Some(RestoredComponent::Experience(state)) if state.owner == Object(actor.raw()) => {
                Ok(true)
            }
            Some(_) => Err("5784C0 Experience slot owner/payload mismatch".into()),
        }
    }
    fn instance_kind(&self, instance: Object) -> Result<u32> {
        Err(format!(
            "5784C0 instance {:#010x}+8 requires the actual saved property-instance arena",
            instance.0
        ))
    }
    fn instance_disabled(&self, instance: Object) -> Result<bool> {
        Err(format!(
            "5784C0 instance {:#010x}+10 requires the actual saved property-instance arena",
            instance.0
        ))
    }
    fn apply(
        &mut self,
        actor: ActorId,
        property: Object,
        kind: u32,
        source: ActorId,
    ) -> Result<()> {
        Err(format!(
            "4CEF10 member {:#010x} property {:#010x} kind {kind} source {:#010x} requires saved admission/instance insertion services",
            actor.raw(),
            property.0,
            source.raw()
        ))
    }
}

//! Live4D1350 and Mind+B4 over canonical actor, property and ability storage.
//! Native action bits and actor state bits are deliberately separate. Remaining
//! recruitment/layout/FX/publication services must be explicitly supplied.
use crate::{
    actor_world::{ActorAccess, RuntimeComponent, Source, Spatial},
    catalog::Catalog,
    definitions::Bank,
};
use aa_actors::{ActorId, ability_flags as flags, components::ComponentSlot};
use aa_mind::{Arena, action_mask as mask, construct::Kind};
use aa_numeric::Precision;
type Result<T> = aa_mind::Result<T>;
fn fail(e: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(format!("live Mind actions: {e}"))
}
fn af(e: impl std::fmt::Display) -> flags::Error {
    flags::Error::World(e.to_string())
}
pub struct Context<'a, 'store, 'source, P: Spatial> {
    pub actors: &'a mut ActorAccess<'store>,
    pub arena: &'a mut Arena,
    pub spatial: &'a mut P,
    pub definitions: &'a Bank,
    pub catalog: &'a Catalog<'source>,
    pub precision: Precision,
}
pub trait Services<P: Spatial> {
    ///Definition-time reads may use a different PC than live arithmetic.
    ///Existing explicit lab contexts retain their previous same-PC contract;
    ///actual loaded callers return their retained source Bank's PC lazily.
    fn source_precision(&self, live:Precision) -> Result<Precision> {Ok(live)}
    fn setting_cache108(&self) -> Result<bool>;
    fn setting_cached8(&self) -> Result<u32>;
    fn state_resupply(&self, index: u8) -> Result<bool>;
    fn organization_member_44(
        &self,
        actor: ActorId,
        actors: &ActorAccess<'_>,
    ) -> Result<Option<ActorId>>;
    fn organization_groups(
        &self,
        actor: ActorId,
        actors: &ActorAccess<'_>,
    ) -> Result<Vec<flags::AttackGroup>>;
    fn owner_can_recruit(
        &mut self,
        owner: ActorId,
        actor: ActorId,
        context: &mut Context<'_, '_, '_, P>,
    ) -> Result<bool>;
    fn owner_recruit_count(
        &mut self,
        owner: ActorId,
        actor: ActorId,
        context: &mut Context<'_, '_, '_, P>,
    ) -> Result<i32>;
    fn mobile_deployment(
        &mut self,
        actor: ActorId,
        entry: u32,
        context: &mut Context<'_, '_, '_, P>,
    ) -> Result<bool>;
    fn other_contributor(
        &mut self,
        actor: ActorId,
        slot: ComponentSlot,
        context: &mut Context<'_, '_, '_, P>,
    ) -> Result<()>;
    fn refresh_owner_views(
        &mut self,
        owner: Option<ActorId>,
        context: &mut Context<'_, '_, '_, P>,
    ) -> Result<()>;
    fn renderer_flags(&self, actor: ActorId, actors: &ActorAccess<'_>) -> Result<Option<u32>>;
    fn refresh_selected_view(
        &mut self,
        actor: ActorId,
        context: &mut Context<'_, '_, '_, P>,
    ) -> Result<()>;
}
pub struct World<'a, 'store, 'source, P: Spatial, S: Services<P>> {
    pub context: Context<'a, 'store, 'source, P>,
    pub services: &'a mut S,
}
fn organization<'a>(
    actors: &'a ActorAccess<'_>,
    actor: ActorId,
) -> Result<&'a aa_actors::ground_states::OrganizationState<String>> {
    match actors
        .component(actor, ComponentSlot::Organization)
        .map_err(fail)?
    {
        Some(RuntimeComponent::Organization(o)) => Ok(o),
        _ => Err(fail("Organization component missing")),
    }
}
fn actor_word(actors: &ActorAccess<'_>, actor: ActorId, offset: u32) -> Result<u32> {
    let key = actors
        .key(actor)
        .ok_or_else(|| fail("actor identity missing"))?;
    actors
        .object(key)
        .map_err(fail)?
        .actor
        .as_ref()
        .ok_or_else(|| fail("ordinary Actor shell missing"))?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| fail(format!("Actor field{offset:#x} uninitialized")))
}
fn build_options_present(actors: &ActorAccess<'_>, actor: ActorId) -> Result<bool> {
    let key = actors.key(actor).ok_or_else(|| fail("actor identity missing"))?;
    Ok(actors.object(key).map_err(fail)?.actor.as_ref()
        .ok_or_else(|| fail("ordinary Actor shell missing"))?
        .actor_list_108.head().is_some())
}
/// Original4D12C0 /4D10A0: source immobility or one of the four blocked states.
pub fn disabled(actors: &ActorAccess<'_>, spatial: &impl Spatial, actor: ActorId) -> Result<bool> {
    let f = actors.action_fields(actor, spatial).map_err(fail)?;
    Ok(f.thing_flags & (1 << 29) != 0
        || f.state_bits & ((1u64 << 34) | (1 << 26) | (1u64 << 36) | (1 << 25)) != 0)
}
fn org_word(actors: &ActorAccess<'_>, actor: ActorId, offset: u32) -> Result<u32> {
    organization(actors, actor)?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| fail(format!("Organization field{offset:#x} uninitialized")))
}
fn kingdom_word(actors: &ActorAccess<'_>, actor: ActorId, offset: u32) -> Result<u32> {
    let key = actors
        .key(actor)
        .ok_or_else(|| fail("Kingdom identity missing"))?;
    let Source::Special(s) = &actors.entity(key).map_err(fail)?.source else {
        return Err(fail("Kingdom runtime missing"));
    };
    if actors.object(key).map_err(fail)?.definition.native_class != 4 {
        return Err(fail("Kingdom native class mismatch"));
    }
    s.derived
        .as_ref()
        .ok_or_else(|| fail("Kingdom payload uninitialized"))?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| fail(format!("Kingdom field{offset:#x} uninitialized")))
}
impl<P: Spatial, S: Services<P>> World<'_, '_, '_, P, S> {
    pub fn rebuild(&mut self, actor: ActorId) -> Result<()> {
        flags::rebuild(self, actor).map_err(fail)
    }
}
impl<P: Spatial, S: Services<P>> flags::World for World<'_, '_, '_, P, S> {
    fn actor_fields(&self, a: ActorId) -> flags::Result<flags::ActorFields> {
        self.context
            .actors
            .action_fields(a, self.context.spatial)
            .map_err(af)
    }
    fn set_action_bits(&mut self, a: ActorId, b: u64) -> flags::Result<()> {
        self.context.actors.set_action_bits(a, b).map_err(af)
    }
    fn has_component(&self, a: ActorId, s: ComponentSlot) -> flags::Result<bool> {
        Ok(self.context.actors.component(a, s).map_err(af)?.is_some())
    }
    fn element(&self, a: ActorId) -> flags::Result<flags::ElementView> {
        match self
            .context
            .actors
            .component(a, ComponentSlot::Element)
            .map_err(af)?
        {
            Some(RuntimeComponent::Element(e)) => Ok(flags::ElementView {
                owner: e.header.owner,
                definition_present: e.element_definition.is_some(),
                organization: e.organization,
                ready: e.flag_1c,
            }),
            _ => Err(af("Element missing")),
        }
    }
    fn organization_member_count(&self, a: ActorId) -> flags::Result<usize> {
        Ok(organization(self.context.actors, a)
            .map_err(af)?
            .members
            .len())
    }
    fn organization_member(&self, a: ActorId, i: usize) -> flags::Result<Option<ActorId>> {
        let id = *organization(self.context.actors, a)
            .map_err(af)?
            .members
            .get(i)
            .ok_or_else(|| af("member index out of native range"))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn organization_member_44(&self, a: ActorId) -> flags::Result<Option<ActorId>> {
        self.services
            .organization_member_44(a, self.context.actors)
            .map_err(af)
    }
    fn organization_reference_d0(&self, a: ActorId) -> flags::Result<ActorId> {
        Ok(ActorId::from_raw(
            org_word(self.context.actors, a, 0xd0).map_err(af)?,
        ))
    }
    fn valid_reference(&self, id: ActorId) -> flags::Result<bool> {
        Ok(self.context.actors.key(id).is_some())
    }
    fn organization_group_count(&self, a: ActorId) -> flags::Result<usize> {
        Ok(self
            .services
            .organization_groups(a, self.context.actors)
            .map_err(af)?
            .len())
    }
    fn organization_attack_group(&self, a: ActorId, i: usize) -> flags::Result<flags::AttackGroup> {
        self.services
            .organization_groups(a, self.context.actors)
            .map_err(af)?
            .get(i)
            .cloned()
            .ok_or_else(|| af("Organization group index out of native range"))
    }
    fn owner(&mut self, a: ActorId) -> flags::Result<Option<ActorId>> {
        self.context.actors.ordinary_owner(a).map_err(af)
    }
    fn kingdom_word_218(&self, a: ActorId) -> flags::Result<u32> {
        kingdom_word(self.context.actors, a, 0x218).map_err(af)
    }
    fn state_resupply(&self, i: u8) -> flags::Result<bool> {
        self.services.state_resupply(i).map_err(af)
    }
    fn owner_can_recruit(&mut self, o: ActorId, a: ActorId) -> flags::Result<bool> {
        self.services
            .owner_can_recruit(o, a, &mut self.context)
            .map_err(af)
    }
    fn owner_recruit_count(&mut self, o: ActorId, a: ActorId) -> flags::Result<i32> {
        self.services
            .owner_recruit_count(o, a, &mut self.context)
            .map_err(af)
    }
    fn action_available(&mut self, a: ActorId, i: u32) -> flags::Result<bool> {
        flags::available(self.actor_fields(a)?.action_bits, i)
    }
    fn other_contributor(&mut self, a: ActorId, s: ComponentSlot) -> flags::Result<()> {
        self.services
            .other_contributor(a, s, &mut self.context)
            .map_err(af)
    }
    fn mind_contributor(&mut self, a: ActorId) -> flags::Result<()> {
        let Some(RuntimeComponent::Mind(m)) = self
            .context
            .actors
            .component(a, ComponentSlot::Mind)
            .map_err(af)?
        else {
            return Err(af("Mind missing"));
        };
        let mind = m.mind_id;
        let kind = m.kind;
        let states = if kind == Kind::Regiment {
            Some(mask::RegimentStates {
                rout: *m
                    .state_bindings
                    .get(&0x20c)
                    .ok_or_else(|| af("Regiment Rout binding missing"))?,
                engage: *m
                    .state_bindings
                    .get(&0x1cc)
                    .ok_or_else(|| af("Regiment Engage binding missing"))?,
            })
        } else {
            None
        };
        let status = if kind == Kind::Regiment {
            Some((
                a.raw(),
                organization_status(self.context.actors, self.context.arena, a).map_err(af)?,
            ))
        } else {
            None
        };
        let mut projection = Projection {
            regiment_status: status,
            actors: self.context.actors,
            spatial: self.context.spatial,
            definitions: self.context.definitions,
            catalog: self.context.catalog,
            precision: self.context.precision,
            services: self.services,
        };
        match kind {
            Kind::Element => mask::element(mind, self.context.arena, &mut projection),
            Kind::Regiment => mask::regiment(
                mind,
                states.unwrap(),
                self.context.arena,
                &mut projection,
                self.context.precision,
            ),
            Kind::Aircraft => mask::aircraft(mind, self.context.arena, &mut projection),
            Kind::Base => Err(fail("abstract Mind action contribution")),
        }
        .map_err(af)
    }
    fn changed(&mut self, a: ActorId) -> flags::Result<()> {
        aa_actors::action_publication::changed(a, self)
    }
}
struct Projection<'a, 'store, 'source, P: Spatial, S: Services<P>> {
    regiment_status: Option<(u32, i32)>,
    actors: &'a mut ActorAccess<'store>,
    spatial: &'a mut P,
    definitions: &'a Bank,
    catalog: &'a Catalog<'source>,
    precision: Precision,
    services: &'a mut S,
}
impl<P: Spatial, S: Services<P>> mask::World for Projection<'_, '_, '_, P, S> {
    fn actor(&self, a: u32) -> Result<mask::Actor> {
        let a = ActorId::from_raw(a);
        let f = self.actors.action_fields(a, self.spatial).map_err(fail)?;
        let has = |s| {
            self.actors
                .component(a, s)
                .map(|x| x.is_some())
                .map_err(fail)
        };
        Ok(mask::Actor {
            flags: f.thing_flags,
            element: has(ComponentSlot::Element)?.then_some(a.raw()),
            organization: has(ComponentSlot::Organization)?.then_some(a.raw()),
            performer: has(ComponentSlot::Performer)?.then_some(a.raw()),
            mover: has(ComponentSlot::Mover)?,
            component9c: has(ComponentSlot::Construction)?,
            // Only nullness is consumed by the native mask coordinator.
            word108: u32::from(build_options_present(self.actors, a)?),
            word118: actor_word(self.actors, a, 0x118)?,
            componentac: has(ComponentSlot::MobileDeployment)?.then_some(a.raw()),
        })
    }
    fn action_bits(&self, a: u32) -> Result<u64> {
        Ok(self
            .actors
            .action_fields(ActorId::from_raw(a), self.spatial)
            .map_err(fail)?
            .action_bits)
    }
    fn set_action_bits(&mut self, a: u32, b: u64) -> Result<()> {
        self.actors
            .set_action_bits(ActorId::from_raw(a), b)
            .map_err(fail)
    }
    fn element_organization(&self, a: u32) -> Result<Option<u32>> {
        match self
            .actors
            .component(ActorId::from_raw(a), ComponentSlot::Element)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Element(e)) => Ok(e.organization.map(ActorId::raw)),
            _ => Err(fail("Element missing")),
        }
    }
    fn abilities(&self, a: u32) -> Result<Vec<mask::Ability>> {
        let a = ActorId::from_raw(a);
        let Some(RuntimeComponent::Performer(p)) = self
            .actors
            .component(a, ComponentSlot::Performer)
            .map_err(fail)?
        else {
            return Err(fail("Performer missing"));
        };
        let mut out = Vec::new();
        for handle in &p.state.abilities {
            let current = self
                .actors
                .abilities
                .get(handle)
                .ok_or_else(|| fail("ability handle missing"))?;
            if !current.enabled {
                out.push(mask::Ability {
                    enabled: false,
                    flags: 0,
                    required_targets: 0,
                    effect_kinds: vec![],
                });
                continue;
            }
            let local = self
                .definitions
                .entry(
                    current
                        .local_definition
                        .ok_or_else(|| fail("ability local definition missing"))?,
                )
                .map_err(fail)?;
            if local.key.domain != "actorlocalability" {
                return Err(fail(
                    "capability source requires ActorLocalAbility identity",
                ));
            }
            let (name, index) = local
                .key
                .name
                .rsplit_once("/ability/")
                .ok_or_else(|| fail("local Ability identity shape"))?;
            let index: usize = index.parse().map_err(fail)?;
            self.catalog.require_untranslated_actor_identity(name).map_err(fail)?;
            let binding = self
                .catalog
                .actor_bindings(name, self.services.source_precision(self.precision)?)
                .map_err(fail)?;
            let loaded = binding
                .performer
                .as_ref()
                .and_then(|p| p.abilities().get(index))
                .ok_or_else(|| fail("loaded Ability source index missing"))?;
            let actual = self.definitions.entry(current.definition).map_err(fail)?;
            crate::saved_organization_actions::require_shared_ability_identity(
                self.catalog,&loaded.authored.shared_definition_id,
            ).map_err(fail)?;
            let (domain, expected) = match &loaded.authored.shared_definition_id {
                Some(aa_rules::native::TypedValue::String { value, .. }) => {
                    ("abilitydefinition", value.to_ascii_lowercase())
                }
                None => ("actorlocalabilitydefinition", local.key.name.clone()),
                _ => return Err(fail("shared ability source type")),
            };
            if actual.key.domain != domain || actual.key.name != expected {
                return Err(fail(
                    "live ability definition changed from its loaded source; resolve replacement before capability query",
                ));
            }
            out.push(mask::Ability {
                enabled: true,
                flags: loaded.flags,
                required_targets: loaded.targets.required,
                effect_kinds: loaded.effects.iter().map(|e| e.kind).collect(),
            });
        }
        Ok(out)
    }
    fn resolve(&self, id: u32) -> Result<Option<u32>> {
        Ok(self.actors.key(ActorId::from_raw(id)).map(|_| id))
    }
    fn organization_link_d0(&self, a: u32) -> Result<u32> {
        org_word(self.actors, ActorId::from_raw(a), 0xd0)
    }
    fn organization_status_578140(&self, a: u32) -> Result<i32> {
        self.regiment_status
            .filter(|(id, _)| *id == a)
            .map(|(_, value)| value)
            .ok_or_else(|| fail("Regiment initial status snapshot identity mismatch"))
    }
    fn organization_values(&self, a: u32) -> Result<[u32; 2]> {
        let a = ActorId::from_raw(a);
        Ok([
            organization(self.actors, a)?.definition_value_308.0,
            org_word(self.actors, a, 0x5c)?,
        ])
    }
    fn setting_cache108(&self) -> Result<bool> {
        self.services.setting_cache108()
    }
    fn setting_cached8(&self) -> Result<u32> {
        self.services.setting_cached8()
    }
    fn actor_player(&mut self, a: u32, _: &mut Arena) -> Result<Option<u32>> {
        Ok(self
            .actors
            .component(ActorId::from_raw(a), ComponentSlot::Properties)
            .map_err(fail)?
            .is_some()
            .then_some(a))
    }
    fn player_flags(&self, a: u32) -> Result<u32> {
        match self
            .actors
            .component(ActorId::from_raw(a), ComponentSlot::Properties)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Properties(p)) => Ok(p.modifier.flags),
            _ => Err(fail("Modifier component missing")),
        }
    }
    fn actor_state(&mut self, a: u32, i: u32, _: &mut Arena) -> Result<bool> {
        flags::available(self.action_bits(a)?, i).map_err(fail)
    }
    fn actor_predicate_4d12c0(&mut self, a: u32, _: &mut Arena) -> Result<bool> {
        disabled(self.actors, self.spatial, ActorId::from_raw(a))
    }
    fn component_ac_predicate(&mut self, a: u32, entry: u32, arena: &mut Arena) -> Result<bool> {
        self.services.mobile_deployment(
            ActorId::from_raw(a),
            entry,
            &mut Context {
                actors: self.actors,
                arena,
                spatial: self.spatial,
                definitions: self.definitions,
                catalog: self.catalog,
                precision: self.precision,
            },
        )
    }
}

/// The native Regiment+B4 calls578140 before any mutating callback. Capture
/// only that pure read at this entry; subsequent fields keep live rereads.
pub(crate) fn organization_status(actors: &ActorAccess<'_>, arena: &Arena, a: ActorId) -> Result<i32> {
    let org = organization(actors, a)?;
    let mind = match actors
        .component(org.header.owner, ComponentSlot::Mind)
        .map_err(fail)?
    {
        Some(RuntimeComponent::Mind(m)) => Some(m),
        None => None,
        _ => return Err(fail("Mind slot kind mismatch")),
    };
    let words = if let Some(m) = mind {
        let core = arena.mind(m.mind_id)?;
        Some([core.words_24_30[1], core.words_24_30[2]])
    } else {
        None
    };
    aa_actors::ability_flags::organization_status(org, words, || {
        let m = mind.ok_or_else(|| af("Organization status Mind missing"))?;
        match m.kind {
            Kind::Regiment => {
                let s = *m
                    .state_bindings
                    .get(&0x270)
                    .ok_or_else(|| af("Regiment exhausted binding missing"))?;
                mask::exhausted_in_stack(m.mind_id, s, arena).map_err(af)
            }
            Kind::Element | Kind::Aircraft => Ok(false),
            Kind::Base => Err(af("abstract Mind exhausted predicate")),
        }
    })
    .map(|v| v as i32)
    .map_err(fail)
}
impl<P: Spatial, S: Services<P>> aa_actors::action_publication::World for World<'_, '_, '_, P, S> {
    fn element_organization(&self, a: ActorId) -> flags::Result<Option<ActorId>> {
        match self
            .context
            .actors
            .component(a, ComponentSlot::Element)
            .map_err(af)?
        {
            Some(RuntimeComponent::Element(e)) => Ok(e.organization),
            None => Ok(None),
            _ => Err(af("Element slot kind mismatch")),
        }
    }
    fn organization_owner(&self, a: ActorId) -> flags::Result<ActorId> {
        Ok(organization(self.context.actors, a)
            .map_err(af)?
            .header
            .owner)
    }
    fn rebuild_actions(&mut self, a: ActorId) -> flags::Result<()> {
        flags::rebuild(self, a)
    }
    fn owner(&mut self, a: ActorId) -> flags::Result<Option<ActorId>> {
        self.context.actors.ordinary_owner(a).map_err(af)
    }
    fn object_byte_4c(&self, a: ActorId) -> flags::Result<u8> {
        let key = self
            .context
            .actors
            .key(a)
            .ok_or_else(|| af("Actor identity missing"))?;
        Ok(self
            .context
            .actors
            .object(key)
            .map_err(af)?
            .object
            .as_ref()
            .ok_or_else(|| af("Object shell missing"))?
            .byte_4c)
    }
    fn refresh_owner_views(&mut self, a: Option<ActorId>) -> flags::Result<()> {
        self.services
            .refresh_owner_views(a, &mut self.context)
            .map_err(af)
    }
    fn renderer_flags(&self, a: ActorId) -> flags::Result<Option<u32>> {
        self.services
            .renderer_flags(a, self.context.actors)
            .map_err(af)
    }
    fn refresh_selected_view(&mut self, a: ActorId) -> flags::Result<()> {
        self.services
            .refresh_selected_view(a, &mut self.context)
            .map_err(af)
    }
}

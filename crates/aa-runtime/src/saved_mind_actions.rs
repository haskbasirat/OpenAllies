//! Saved Mind virtual+B4 (530510/521E10/518720) over the actual owned core,
//! state identities, current components and Ability allocations. The core is
//! moved, never cloned or reconstructed; all commands and raw saved sources
//! stay in RestoredMindParts, and every error returns ownership to its slot.
use crate::{catalog::RestoredMindParts, saved_organization_actions::SavedActions};
use aa_actors::{ActorId, ability_flags as flags, components::ComponentSlot};
use aa_mapworld::{restore::RestoredComponent, restore_components::RestoredMind};
use aa_mind::{
    Arena, action_mask as mask,
    construct::{Installed, Kind},
};

type Result<T> = aa_mind::Result<T>;
fn fail(error: impl std::fmt::Display) -> aa_mind::Error {
    aa_mind::Error::World(format!("saved Mind actions: {error}"))
}

pub(crate) fn contribute(actions: &mut SavedActions<'_, '_, '_>, actor: ActorId) -> Result<()> {
    if actions.active_mind.is_some() {
        return Err(fail("nested action arena ownership is unsupported"));
    }
    let Some(RestoredComponent::Mind(component)) = actions
        .component(actor, ComponentSlot::Mind)
        .map_err(fail)?
    else {
        return Err(fail("Mind component missing"));
    };
    let parts = component
        .state
        .as_any()
        .downcast_ref::<RestoredMindParts>()
        .ok_or_else(|| fail("Mind payload is not the original restored allocation"))?;
    let core = parts
        .mind
        .as_ref()
        .ok_or_else(|| fail("Mind core is absent"))?;
    if core.actor != actor.raw() {
        return Err(fail("Mind owner identity mismatch"));
    }
    // Validate before taking ownership. No fallible callback can strand the
    // component after removal: the native masks cannot erase actors or cores.
    let id = parts.installed.mind_id;
    let kind = parts.installed.kind;
    let states = if kind == Kind::Regiment {
        Some(mask::RegimentStates {
            rout: *parts
                .installed
                .state_bindings
                .get(&0x20c)
                .ok_or_else(|| fail("Regiment Rout binding missing"))?,
            engage: *parts
                .installed
                .state_bindings
                .get(&0x1cc)
                .ok_or_else(|| fail("Regiment Engage binding missing"))?,
        })
    } else {
        None
    };
    let Some(RestoredComponent::Mind(component)) = actions
        .shell_mut(actor)
        .map_err(fail)?
        .components
        .remove(ComponentSlot::Mind)
    else {
        unreachable!("validated Mind slot")
    };
    let required = component.required;
    let mut parts = component
        .state
        .into_any()
        .downcast::<RestoredMindParts>()
        .expect("validated payload type");
    let mut arena = Arena {
        minds: [(id, parts.mind.take().expect("validated core"))].into(),
        states: parts.states,
    };
    actions.active_mind = Some(actor);
    let result = (|| {
        let status = if kind == Kind::Regiment {
            Some((
                actor.raw(),
                organization_status(actions, &arena, actor, &parts.installed)?,
            ))
        } else {
            None
        };
        let precision = actions.context.precision;
        let mut projection = Projection {
            actions,
            regiment_status: status,
        };
        match kind {
            Kind::Element => mask::element(id, &mut arena, &mut projection),
            Kind::Regiment => mask::regiment(
                id,
                states.expect("validated bindings"),
                &mut arena,
                &mut projection,
                precision,
            ),
            Kind::Aircraft => mask::aircraft(id, &arena, &mut projection),
            Kind::Base => Err(fail("abstract Mind action contribution")),
        }
    })();
    parts.mind = Some(
        arena
            .minds
            .remove(&id)
            .expect("action mask retains its owned Mind"),
    );
    parts.states = arena.states;
    actions.active_mind = None;
    let previous = actions
        .shell_mut(actor)
        .expect("action mask cannot remove its actor")
        .components
        .insert(
            ComponentSlot::Mind,
            RestoredComponent::Mind(RestoredMind {
                required,
                state: parts,
            }),
        );
    assert!(
        previous.is_none(),
        "action mask cannot replace its owned Mind"
    );
    result
}

fn organization_status(
    actions: &SavedActions<'_, '_, '_>,
    arena: &Arena,
    actor: ActorId,
    installed: &Installed,
) -> Result<i32> {
    let org = actions.org(actor).map_err(fail)?;
    if org.header.owner != actor {
        return Err(fail("Organization owner identity mismatch"));
    }
    let core = arena.mind(installed.mind_id)?;
    flags::organization_status(
        org,
        Some([core.words_24_30[1], core.words_24_30[2]]),
        || {
            let exhausted = *installed
                .state_bindings
                .get(&0x270)
                .ok_or_else(|| flags::Error::World("Regiment exhausted binding missing".into()))?;
            mask::exhausted_in_stack(installed.mind_id, exhausted, arena)
                .map_err(|e| flags::Error::World(e.to_string()))
        },
    )
    .map(|value| value as i32)
    .map_err(fail)
}

struct Projection<'a, 'catalog, 'world, 'source> {
    actions: &'a mut SavedActions<'catalog, 'world, 'source>,
    regiment_status: Option<(u32, i32)>,
}
impl mask::World for Projection<'_, '_, '_, '_> {
    fn actor(&self, id: u32) -> Result<mask::Actor> {
        let actor = ActorId::from_raw(id);
        let fields = flags::World::actor_fields(self.actions, actor).map_err(fail)?;
        let has = |slot| flags::World::has_component(self.actions, actor, slot).map_err(fail);
        Ok(mask::Actor {
            flags: fields.thing_flags,
            element: has(ComponentSlot::Element)?.then_some(id),
            organization: has(ComponentSlot::Organization)?.then_some(id),
            performer: has(ComponentSlot::Performer)?.then_some(id),
            mover: has(ComponentSlot::Mover)?,
            component9c: has(ComponentSlot::Construction)?,
            // The action-mask view only tests native +108 for null. Project
            // actual typed-list presence, never a logical NodeId as a pointer.
            word108: u32::from(
                self.actions
                    .shell(actor)
                    .map_err(fail)?
                    .actor_list_108
                    .head()
                    .is_some(),
            ),
            word118: self.actions.word(actor, 0x118).map_err(fail)?,
            componentac: has(ComponentSlot::MobileDeployment)?.then_some(id),
        })
    }
    fn action_bits(&self, id: u32) -> Result<u64> {
        Ok(
            flags::World::actor_fields(self.actions, ActorId::from_raw(id))
                .map_err(fail)?
                .action_bits,
        )
    }
    fn set_action_bits(&mut self, id: u32, bits: u64) -> Result<()> {
        flags::World::set_action_bits(self.actions, ActorId::from_raw(id), bits).map_err(fail)
    }
    fn element_organization(&self, id: u32) -> Result<Option<u32>> {
        Ok(flags::World::element(self.actions, ActorId::from_raw(id))
            .map_err(fail)?
            .organization
            .map(ActorId::raw))
    }
    fn abilities(&self, id: u32) -> Result<Vec<mask::Ability>> {
        let Some(RestoredComponent::Performer(performer)) = self
            .actions
            .component(ActorId::from_raw(id), ComponentSlot::Performer)
            .map_err(fail)?
        else {
            return Err(fail("Performer component missing"));
        };
        if performer.state.owner != id {
            return Err(fail("Performer owner identity mismatch"));
        }
        let arena = self
            .actions
            .catalog
            .ability_arena
            .try_borrow()
            .map_err(fail)?;
        let bank = self
            .actions
            .catalog
            .definition_bank
            .try_borrow()
            .map_err(fail)?;
        let mut out = Vec::with_capacity(performer.state.abilities.len());
        for &handle in &performer.state.abilities {
            let current = arena.get(handle).map_err(fail)?;
            if !current.enabled {
                out.push(mask::Ability {
                    enabled: false,
                    flags: 0,
                    required_targets: 0,
                    effect_kinds: vec![],
                });
                continue;
            }
            let local = bank
                .entry(
                    current
                        .local_definition
                        .ok_or_else(|| fail("Ability local definition missing"))?,
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
            self.actions
                .catalog
                .require_untranslated_actor_identity(name)
                .map_err(fail)?;
            let binding = self
                .actions
                .catalog
                .actor_bindings(name, self.actions.source_precision().map_err(fail)?)
                .map_err(fail)?;
            let loaded = binding
                .performer
                .as_ref()
                .and_then(|p| p.abilities().get(index))
                .ok_or_else(|| fail("loaded Ability source index missing"))?;
            let actual = bank.entry(current.definition).map_err(fail)?;
            crate::saved_organization_actions::require_shared_ability_identity(
                self.actions.catalog,
                &loaded.authored.shared_definition_id,
            )
            .map_err(fail)?;
            let (domain, expected) = match &loaded.authored.shared_definition_id {
                Some(aa_rules::native::TypedValue::String { value, .. }) => {
                    ("abilitydefinition", value.to_ascii_lowercase())
                }
                None => ("actorlocalabilitydefinition", local.key.name.clone()),
                _ => return Err(fail("shared Ability source type")),
            };
            if actual.key.domain != domain || actual.key.name != expected {
                return Err(fail("Ability definition differs from its loaded source"));
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
        Ok(
            flags::World::valid_reference(self.actions, ActorId::from_raw(id))
                .map_err(fail)?
                .then_some(id),
        )
    }
    fn organization_link_d0(&self, id: u32) -> Result<u32> {
        self.actions
            .org_word(ActorId::from_raw(id), 0xd0)
            .map_err(fail)
    }
    fn organization_status_578140(&self, id: u32) -> Result<i32> {
        self.regiment_status
            .filter(|(owner, _)| *owner == id)
            .map(|(_, value)| value)
            .ok_or_else(|| fail("Regiment initial status identity mismatch"))
    }
    fn organization_values(&self, id: u32) -> Result<[u32; 2]> {
        let actor = ActorId::from_raw(id);
        Ok([
            self.actions
                .org(actor)
                .map_err(fail)?
                .definition_value_308
                .0,
            self.actions.org_word(actor, 0x5c).map_err(fail)?,
        ])
    }
    fn setting_cache108(&self) -> Result<bool> {
        crate::saved_action_settings::fog_of_war(
            self.actions.catalog,
            self.actions.context.inventory,
            self.actions.source_precision().map_err(fail)?,
        )
        .map_err(fail)
    }
    fn setting_cached8(&self) -> Result<u32> {
        crate::saved_action_settings::morale_retreat_penalty(
            self.actions.catalog,
            self.actions.context.inventory,
            self.actions.source_precision().map_err(fail)?,
        )
        .map_err(fail)
    }
    fn actor_player(&mut self, id: u32, _: &mut Arena) -> Result<Option<u32>> {
        match self
            .actions
            .component(ActorId::from_raw(id), ComponentSlot::Properties)
            .map_err(fail)?
        {
            Some(RestoredComponent::Properties(value)) if value.owner.0 == id => Ok(Some(id)),
            None => Ok(None),
            _ => Err(fail("Properties slot payload mismatch")),
        }
    }
    fn player_flags(&self, id: u32) -> Result<u32> {
        match self
            .actions
            .component(ActorId::from_raw(id), ComponentSlot::Properties)
            .map_err(fail)?
        {
            Some(RestoredComponent::Properties(value)) if value.owner.0 == id => {
                Ok(value.modifier.flags)
            }
            _ => Err(fail("Properties Modifier missing")),
        }
    }
    fn actor_state(&mut self, id: u32, index: u32, _: &mut Arena) -> Result<bool> {
        flags::available(self.action_bits(id)?, index).map_err(fail)
    }
    fn actor_predicate_4d12c0(&mut self, id: u32, _: &mut Arena) -> Result<bool> {
        let fields =
            flags::World::actor_fields(self.actions, ActorId::from_raw(id)).map_err(fail)?;
        Ok(fields.thing_flags & (1 << 29) != 0
            || fields.state_bits & ((1u64 << 34) | (1 << 26) | (1u64 << 36) | (1 << 25)) != 0)
    }
    fn component_ac_predicate(&mut self, actor: u32, entry: u32, _: &mut Arena) -> Result<bool> {
        let actor = ActorId::from_raw(actor);
        match entry {
            0x575bb0 => aa_actors::mobile_deployment_actions::can_unpack(self.actions, actor, true)
                .map_err(fail),
            0x575c00 => aa_actors::mobile_deployment_actions::can_pack(
                self.actions,
                actor,
                true,
                self.actions.context.precision,
            )
            .map_err(fail),
            _ => Err(fail("unexpected MobileDeployment predicate")),
        }
    }
}

#[cfg(test)]
#[path = "saved_mind_actions_tests.rs"]
mod tests;

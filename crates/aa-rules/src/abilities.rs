//! Typed authored Ability inputs. Handles, animations and effect execution remain
//! the responsibility of their native-equivalent runtime loaders.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, ScalarType, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct AnimationBinding {
    pub name: String,
    pub length_bits: u32,
    pub use_fraction_bits: u32,
    pub sources: Vec<Provenance>,
}
/// Source-bound Character animation data, before rendered KF playback.
/// Implementations must preserve native set order and lookup semantics.
pub trait AnimationResolver {
    fn set_count(&self) -> u32;
    fn lookup(&self, set: u32, name: &str, expected_kind: u32) -> Result<Option<AnimationBinding>>;
}

#[derive(Clone, Debug, Serialize)]
pub struct EffectInputs {
    pub fields: BTreeMap<String, TypedValue>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AbilityDefinitionInputs {
    pub fields: BTreeMap<String, TypedValue>,
    pub flags: BTreeMap<String, TypedValue>,
    pub effects: Vec<EffectInputs>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct AbilityInputs {
    pub fields: BTreeMap<String, TypedValue>,
    pub definition: AbilityDefinitionInputs,
    /// Present only for the original shared-definition branch; no native pointer
    /// or registry ordinal is synthesized from this authored ID.
    pub shared_definition_id: Option<TypedValue>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DefenderInputs {
    /// ActorData+218 `control_range`.
    pub control_range_bits: u32,
    pub abilities: Vec<AbilityInputs>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct PerformerInputs {
    pub fields: BTreeMap<String, TypedValue>,
    pub abilities: Vec<AbilityInputs>,
    pub sources: Vec<Provenance>,
}
fn read(
    node: &ResolvedNode<'_>,
    names: &[&str],
    pc: Precision,
) -> Result<BTreeMap<String, TypedValue>> {
    names
        .iter()
        .map(|&name| {
            Ok((
                name.to_owned(),
                node.field(name, pc)?.ok_or_else(|| {
                    Error::Unsupported(format!("required Ability field {name} absent"))
                })?,
            ))
        })
        .collect()
}
fn fixed(
    node: &ResolvedNode<'_>,
    names: &[&str],
    pc: Precision,
) -> Result<BTreeMap<String, TypedValue>> {
    let fields = read(node, names, pc)?;
    for (name, value) in &fields {
        if !matches!(value,TypedValue::Scalar(s)if s.field_type==ScalarType::Fixed) {
            return Err(Error::Unsupported(format!(
                "Ability field {name} must be fixed"
            )));
        }
    }
    Ok(fields)
}
fn text(value: &TypedValue) -> Result<&str> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(Error::Unsupported(
            "Ability string has wrong declared type".into(),
        )),
    }
}
/// Native ability effect kinds (498D00 enum table, proven by
/// research/phase8-combat/native-ability-effect-fixtures.json): the index is
/// the effect `+4` kind word. Lookup is ASCII case-insensitive (405030 hash,
/// _wcsicmp); an unknown name throws "Unknown ability effect type".
pub fn effect_kind(name: &str) -> Result<u32> {
    crate::commander::EFFECT_KINDS
        .iter()
        .position(|kind| kind.eq_ignore_ascii_case(name))
        .map(|kind| kind as u32)
        .ok_or_else(|| {
            Error::Unsupported(format!(
                "Unknown ability effect type '{name}' loading ability definition (native throws)"
            ))
        })
}
/// The authored fields one `Effect` node's LoadData reads (4BEF80 dispatch,
/// then the kind's LoadData). Also used for the nested `Effect` of an
/// ABILITY_EFFECT modifier (501150 -> 4CA880).
pub(crate) fn effect_inputs(effect: &ResolvedNode<'_>, pc: Precision) -> Result<EffectInputs> {
    let mut fields = read(effect, &["type", "targets", "hide_from_tooltip"], pc)?;
    fields["hide_from_tooltip"].boolean()?;
    // 4BEF80: case-insensitive effect type enum (498D00 table), then the
    // kind's LoadData reads `targets`, `hide_from_tooltip` and its fields.
    match effect_kind(text(&fields["type"])?)? {
        // 4CC890
        0 => {
            fields.extend(read(
                effect,
                &[
                    "object_ids",
                    "dynamic_campaign_object_ids",
                    "property_ids",
                    "controllable",
                    "unique_target_flags",
                ],
                pc,
            )?);
            fields["controllable"].boolean()?;
        }
        // 4CA010 (MORALE_LOSS/MORALE_GAIN)
        1 | 2 => fields.extend(fixed(effect, &["amount"], pc)?),
        // 4CAB50
        3 => {
            fields.extend(fixed(effect, &["amount"], pc)?);
            fields.extend(read(effect, &["heal_type"], pc)?);
        }
        // 4CCC70 (MAIN_DAMAGE/ADD_DAMAGE)
        4 | 5 => {
            fields.extend(fixed(effect, &["amount"], pc)?);
            fields.extend(read(effect, &["damage_type_IDS"], pc)?);
        }
        // 4CBBF0 (EnchantmentProperty child), 4C9FD0 (CAPTURE,
        // DESCRIBE_ECONOMY), 4CA190 (ResourcesAmount child): the children
        // are loaded by ability_load.
        _ => {}
    }
    Ok(EffectInputs {
        fields,
        sources: effect.sources(),
    })
}
fn definition(node: &ResolvedNode<'_>, pc: Precision) -> Result<AbilityDefinitionInputs> {
    let mut fields = fixed(
        node,
        &[
            "min_range",
            "recharge_time",
            "range",
            "area",
            "volley_area",
            "mana_cost",
            "impact_impulse",
            "recoil_impulse",
            "attenuation_range_min",
            "attenuation_factor_max",
            "impulse_area",
        ],
        pc,
    )?;
    fields.extend(read(
        node,
        &[
            "IDS",
            "ability_type",
            "targets",
            "projectile_IDS",
            "max_targets",
        ],
        pc,
    )?);
    let flags_node = node
        .child("Flags")?
        .ok_or_else(|| Error::Unsupported("Ability Flags absent".into()))?;
    let flags = read(
        &flags_node,
        &[
            "spell",
            "vorpal",
            "enchantment",
            "bombard",
            "center_on_source",
            "target_location",
            "heal",
            "ignore_for_cv",
            "summon_multiple",
            "summon_unique",
            "primary_island_only",
        ],
        pc,
    )?;
    for value in flags.values() {
        value.boolean()?;
    }
    let mut effects = Vec::new();
    for effect in node.children("Effect")? {
        effects.push(effect_inputs(&effect, pc)?);
    }
    Ok(AbilityDefinitionInputs {
        fields,
        flags,
        effects,
        sources: node.sources(),
    })
}
impl ActorCatalog {
    /// One authored `Ability` child, as read by 4BE9B0/4BF990 for both the
    /// PerformerComponent and DefenderComponent lists.
    pub fn native_ability_inputs(
        &self,
        ability: &ResolvedNode<'_>,
        pc: Precision,
    ) -> Result<AbilityInputs> {
        let mut fields = fixed(
            ability,
            &[
                "reload_time",
                "priority",
                "move_weight",
                "use_weight",
                "wait_weight",
                "move_bonus",
                "use_bonus",
                "wait_bonus",
                "fuzzy_range",
            ],
            pc,
        )?;
        fields.extend(read(
            ability,
            &[
                "spawn_position",
                "spawn_bone_ids",
                "animation1",
                "animation2",
                "animation3",
                "animation4",
                "animation5",
                "idle_animation",
                "turret_ids",
                "required_properties",
                "required_experience_min",
                "required_experience_max",
                "cost_faction_experience",
            ],
            pc,
        )?);
        let (definition_node, shared_definition_id) =
            if let Some(local) = ability.child("AbilityDefinition")? {
                (local, None)
            } else {
                let value = ability.field("ability_IDS", pc)?.ok_or_else(|| {
                    Error::Unsupported("Ability has no local or shared definition".into())
                })?;
                (
                    self.resolved_definition("AbilityDefinition", text(&value)?, pc)?,
                    Some(value),
                )
            };
        Ok(AbilityInputs {
            fields,
            definition: definition(&definition_node, pc)?,
            shared_definition_id,
            sources: ability.sources(),
        })
    }
    /// 464BD0 DefenderComponent (4657A0..465937): `control_range` (+218) and
    /// the ordered Ability list (+21C) of local abilities.
    pub fn native_defender_inputs(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Option<DefenderInputs>> {
        let Some(node) = self.resolved_actor(id)?.child("DefenderComponent")? else {
            return Ok(None);
        };
        let control_range_bits = node
            .field("control_range", pc)?
            .ok_or_else(|| {
                Error::Unsupported("Defender control_range missing (native throws)".into())
            })?
            .fixed_bits()?;
        let mut abilities = Vec::new();
        for ability in node.children("Ability")? {
            abilities.push(self.native_ability_inputs(&ability, pc)?);
        }
        Ok(Some(DefenderInputs {
            control_range_bits,
            abilities,
            sources: node.sources(),
        }))
    }
    /// Source snapshots do not constitute successful native model/effect/target
    /// handle loading and must not be passed as completed Performer count.
    pub fn native_performer_inputs(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Option<PerformerInputs>> {
        let Some(node) = self.resolved_actor(id)?.child("PerformerComponent")? else {
            return Ok(None);
        };
        let mut fields = fixed(
            &node,
            &[
                "mana_max",
                "mana_regeneration",
                "attack_height",
                "move_bonus",
                "use_bonus",
                "wait_bonus",
                "cv_bias",
            ],
            pc,
        )?;
        fields.extend(read(&node, &["check_damage_resistance"], pc)?);
        let mut abilities = Vec::new();
        for ability in node.children("Ability")? {
            abilities.push(self.native_ability_inputs(&ability, pc)?);
        }
        Ok(Some(PerformerInputs {
            fields,
            abilities,
            sources: node.sources(),
        }))
    }
}
#[path = "ability_load.rs"]
mod loading;
pub use loading::*;

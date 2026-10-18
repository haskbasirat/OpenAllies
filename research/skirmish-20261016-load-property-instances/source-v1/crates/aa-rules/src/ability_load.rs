//! Checked bounded gameplay definition loader for original ground abilities.
use super::{AbilityInputs, AnimationBinding, AnimationResolver, PerformerInputs};
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
    resources::Provenance,
    targets::TargetFilter,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize)]
pub struct DefinitionIdentity {
    pub kind: String,
    pub id: String,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct DenseCost {
    pub resource_ids: Vec<String>,
    pub bits: Vec<u32>,
    pub fields: BTreeMap<String, TypedValue>,
    pub resource_sources: Vec<Vec<Provenance>>,
}
#[derive(Clone, Debug, Serialize)]
pub struct LoadedAnimationSet {
    pub idle: AnimationBinding,
    pub attacks: Vec<AnimationBinding>,
}
/// AbilityDefinition `Flags` bits (+54 bits 0..=10) in native order.
pub const DEFINITION_FLAGS: [&str; 11] = [
    "vorpal",
    "spell",
    "enchantment",
    "bombard",
    "center_on_source",
    "target_location",
    "heal",
    "ignore_for_cv",
    "summon_multiple",
    "summon_unique",
    "primary_island_only",
];
/// Native heal type enum (4CC1D0 table): INVALID 0, HEAL 1, REPAIR 2.
pub const HEAL_TYPES: [&str; 3] = ["INVALID", "HEAL", "REPAIR"];
/// Native FXC type enum (498C30 table); only CREATE_VFX (0) is consumed by
/// the enchantment permanent-VFX check.
pub const FXC_TYPES: [&str; 10] = [
    "CREATE_VFX",
    "STOP_VFX",
    "EQUIPMENT",
    "PLAY_SFX",
    "STOP_SFX",
    "CHANGE_ASPECT",
    "CHANGE_SKIN",
    "CREATE_RFX",
    "PLAY_OBJECT_ANIMATION",
    "PLAY_HIT_EFFECT",
];

/// Native modifier type enum (4C1060 table, map B3B654).
pub const MODIFIER_TYPES: [&str; 10] = [
    "FLAG",
    "ADD",
    "MULTIPLY",
    "ENCHANTMENT_IMMUNITY",
    "ABILITY_EFFECT",
    "RESIST_DAMAGE",
    "PERIODIC_DAMAGE",
    "PERIODIC_HEAL",
    "REVERSE_ABILITY",
    "RESOURCE_PRODUCTION",
];
/// Loaded enchantment-property modifiers (4C4030 per kind).
#[derive(Clone, Debug, Serialize)]
pub enum EnchantmentModifier {
    Flag(crate::property_inputs::FlagModifierInputs),
    /// 4FEC70: +C amount, +10 attribute (0..=12), +14 bit 0 repair.
    Add {
        attribute: u32,
        amount_bits: u32,
        repair: bool,
        sources: Vec<Provenance>,
    },
    ResistDamage {
        amount_bits: u32,
        damage_type: DefinitionIdentity,
        damage_type_index: u32,
        sources: Vec<Provenance>,
    },
    /// 501350: +C damage type, +10 amount.
    PeriodicDamage {
        damage_type: DefinitionIdentity,
        damage_type_index: u32,
        amount_bits: u32,
        sources: Vec<Provenance>,
    },
    /// 4FED70: +C amount (negated for 18/23), +10 attribute (13..=24).
    Multiply {
        attribute: u32,
        amount_bits: u32,
        sources: Vec<Provenance>,
    },
    /// 5010A0: +C EnchantmentType (null when `enchantment_type` is empty).
    EnchantmentImmunity {
        enchantment_type: Option<DefinitionIdentity>,
        sources: Vec<Provenance>,
    },
    /// 501150: +C AbilityType, +10 nested effect (4CA880 + its LoadData).
    AbilityEffect {
        ability_type: DefinitionIdentity,
        effect: Box<LoadedEffect>,
        sources: Vec<Provenance>,
    },
    /// 4FEE90: +C amount.
    PeriodicHeal {
        amount_bits: u32,
        sources: Vec<Provenance>,
    },
    /// 501410: +C AbilityDefinition.
    ReverseAbility {
        ability: DefinitionIdentity,
        sources: Vec<Provenance>,
    },
    /// 4FEEC0: +C dense production in Resource registry order (+10 count).
    ResourceProduction {
        resource_ids: Vec<String>,
        bits: Vec<u32>,
        sources: Vec<Provenance>,
    },
}
/// EnchantmentProperty (vtable A25F6C) loaded by 4CBBF0 -> 4C4F90.
#[derive(Clone, Debug, Serialize)]
pub struct EnchantmentProperty {
    /// `ids` (constructor 47D8D0); registered in the Property registry by
    /// 4B4E60 ("Data IDS already used" when it collides).
    pub id: String,
    pub sources: Vec<Provenance>,
    /// +5C.
    pub is_technology: bool,
    /// +60.
    pub duration_bits: u32,
    /// +64: EnchantmentType registry entry ([B36530]+3AC).
    pub enchantment_type: DefinitionIdentity,
    /// 4C42D0 modifiers in authored order (4C4030 per kind).
    pub modifiers: Vec<EnchantmentModifier>,
    /// +38 required properties (4C4E10 deferred resolution), in native
    /// list (reverse authored) order.
    pub required_properties: Vec<DefinitionIdentity>,
    /// +24 enabled target flag word.
    pub target_flags: u32,
    pub metadata: BTreeMap<String, TypedValue>,
    pub fx_sources: BTreeMap<String, Vec<Vec<Provenance>>>,
    /// WhenAdded CREATE_VFX entries satisfying 428360 (authored depth-first
    /// order; 428A30 needs at least one).
    pub permanent_vfx: Vec<DefinitionIdentity>,
}

/// Kind-specific effect payload, as stored by each LoadData.
#[derive(Clone, Debug, Serialize)]
pub enum EffectData {
    /// MORALE_LOSS / MORALE_GAIN (4CA010): +14 amount.
    Morale { amount_bits: u32 },
    /// HEAL (4CAB50): +14 amount, +18 heal type.
    Heal { amount_bits: u32, heal_type: u32 },
    /// MAIN_DAMAGE / ADD_DAMAGE (4CCC70): +14 damage type, +18 amount.
    Damage {
        damage_type: DefinitionIdentity,
        amount_bits: u32,
    },
    /// ENCHANTMENT (4CBBF0): +14 property.
    Enchantment(Box<EnchantmentProperty>),
    /// CAPTURE / DESCRIBE_ECONOMY (4C9FD0): base fields only.
    Plain,
    /// STEAL_RESOURCES (4CA190): +14 dense ResourcesAmount (473CB0).
    StealResources(DenseCost),
    /// SUMMON (4CC890 + 4CC730).
    Summon(Box<SummonEffectData>),
}
/// SummonEffect after LoadData 4CC890 and its deferred resolution 4CC730.
#[derive(Clone, Debug, Serialize)]
pub struct SummonEffectData {
    /// +14 `object_ids` through 427BD0 (Thing manager, translations applied).
    pub object: DefinitionIdentity,
    /// +18 `dynamic_campaign_object_ids`, or `object` when that is empty.
    pub dynamic_object: DefinitionIdentity,
    pub object_facts: SummonObjectFacts,
    pub dynamic_facts: SummonObjectFacts,
    /// +24 `property_ids` (immediate 463FA0 Property lookup).
    pub property: Option<DefinitionIdentity>,
    /// +28.
    pub controllable: bool,
    /// +1C `unique_target_flags` (4C5BF0).
    pub unique_targets: TargetFilter,
}
#[derive(Clone, Debug, Serialize)]
pub struct LoadedEffect {
    /// +4 effect kind (`commander::EFFECT_KINDS` index).
    pub kind: u32,
    /// +8 target filter.
    pub targets: TargetFilter,
    /// +10.
    pub hide_from_tooltip: bool,
    pub data: EffectData,
}
#[derive(Clone, Debug, Serialize)]
pub struct AbilityGeometry {
    pub attenuation_min_bits: u32,
    pub attenuation_factor_bits: u32,
    pub impulse_area_bits: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct LoadedAbility {
    pub authored: AbilityInputs,
    pub targets: TargetFilter,
    pub flags: u32,
    pub power_bits: u32,
    pub geometry: AbilityGeometry,
    pub effects: Vec<LoadedEffect>,
    pub animations: Vec<LoadedAnimationSet>,
    pub recharge_bits: u32,
    pub costs: DenseCost,
    pub faction_xp_cost: i32,
    pub spawn_position_bits: [u32; 3],
    pub spawn_bone: Option<DefinitionIdentity>,
    pub projectile: Option<DefinitionIdentity>,
    pub ability_type: Option<DefinitionIdentity>,
    /// AbilityData+54: ActorData turret index from `turret_ids`, or -1.
    pub turret_index: i32,
    /// AbilityData+58 required properties, in native list (reverse) order.
    pub required_properties: Vec<DefinitionIdentity>,
}
#[derive(Clone, Debug, Serialize)]
pub struct LoadedDefender {
    pub authored: super::DefenderInputs,
    abilities: Vec<LoadedAbility>,
}
impl LoadedDefender {
    pub fn abilities(&self) -> &[LoadedAbility] {
        &self.abilities
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct LoadedPerformer {
    pub authored: PerformerInputs,
    abilities: Vec<LoadedAbility>,
}
impl LoadedPerformer {
    pub fn abilities(&self) -> &[LoadedAbility] {
        &self.abilities
    }
    pub fn completed_ability_count(&self) -> u32 {
        self.abilities.len() as u32
    }
}
fn missing(name: &str) -> Error {
    Error::Unsupported(format!("required gameplay input {name} missing"))
}
fn value<'a>(fields: &'a BTreeMap<String, TypedValue>, name: &str) -> Result<&'a TypedValue> {
    fields.get(name).ok_or_else(|| missing(name))
}
fn string(fields: &BTreeMap<String, TypedValue>, name: &str) -> Result<String> {
    match value(fields, name)? {
        TypedValue::String { value, .. } => Ok(value.clone()),
        _ => Err(Error::Unsupported(format!("{name} must be native string"))),
    }
}
fn fixed(fields: &BTreeMap<String, TypedValue>, name: &str) -> Result<u32> {
    value(fields, name)?.fixed_bits()
}
fn identity(
    catalog: &ActorCatalog,
    kind: &str,
    id: &str,
    pc: Precision,
) -> Result<DefinitionIdentity> {
    let node = if kind == "Thing" {
        catalog.resolved_actor(id)?
    } else {
        catalog.resolved_definition(kind, id, pc)?
    };
    Ok(DefinitionIdentity {
        kind: kind.into(),
        id: id.to_owned(),
        sources: node.sources(),
    })
}
/// Original4BFF32..4BFFCA, with extended sum compared before its f32 store.
pub fn derived_recharge(length: u32, reload: u32, definition: u32, pc: Precision) -> Result<u32> {
    let sum = Binary::from_f32_bits(length)?.add(&Binary::from_f32_bits(reload)?, pc)?;
    let minimum = Binary::from_f32_bits(definition)?;
    Ok(if minimum.compare(&sum) == Ordering::Greater {
        minimum
    } else {
        sum
    }
    .store_f32()?)
}
/// Original4BEE20 walks the loaded effects, preserving main amount overwrite
/// behavior when an earlier main amount was zero or negative.
pub fn effect_flags_and_power(flags: u32, effects: &[(u32, u32)]) -> Result<(u32, u32)> {
    let mut flags = flags & 0x7ff;
    let mut power = 0;
    for &(kind, amount) in effects {
        flags |= 1u32.wrapping_shl(kind.wrapping_add(11));
        if kind == 4 {
            if Binary::from_f32_bits(power)?.compare(&Binary::from_u32(0)) == Ordering::Greater {
                return Err(Error::Unsupported(
                    "native Ability rejects a second main damage effect after positive main power"
                        .into(),
                ));
            }
            Binary::from_f32_bits(amount)?;
            power = amount;
        }
    }
    Ok((flags, power))
}
/// Original4BF54C..4BF67F. Non-area definitions retain the constructor's
/// zero words instead of copying unused authored attenuation defaults.
pub fn ability_geometry(
    area: u32,
    minimum: u32,
    factor: u32,
    impulse: u32,
    pc: Precision,
) -> Result<AbilityGeometry> {
    let zero = Binary::from_u32(0);
    let one = Binary::from_u32(1);
    let a = Binary::from_f32_bits(area)?;
    if a.compare(&zero) != Ordering::Greater {
        return Ok(AbilityGeometry {
            attenuation_min_bits: 0,
            attenuation_factor_bits: 0,
            impulse_area_bits: 0,
        });
    }
    let min = Binary::from_f32_bits(minimum)?;
    let f = Binary::from_f32_bits(factor)?;
    let i = Binary::from_f32_bits(impulse)?;
    let epsilon = Binary::from_f32_bits(0x38d1b717)?;
    if min.compare(&zero) == Ordering::Less
        || min.compare(&a.add(&epsilon, pc)?) == Ordering::Greater
    {
        return Err(Error::Unsupported("invalid attenuation minimum".into()));
    }
    if f.compare(&zero) == Ordering::Less
        || f.compare(&Binary::from_f32_bits(0x3f800347)?) == Ordering::Greater
    {
        return Err(Error::Unsupported("invalid attenuation factor".into()));
    }
    if i.compare(&zero) == Ordering::Less {
        return Err(Error::Unsupported("negative impulse area".into()));
    }
    let attenuation_min_bits =
        if min.compare(&epsilon) != Ordering::Greater || min.compare(&a) == Ordering::Greater {
            area
        } else {
            minimum
        };
    let attenuation_factor_bits = if f.compare(&zero) == Ordering::Less {
        0
    } else if f.compare(&one) == Ordering::Greater {
        0x3f800000
    } else {
        factor
    };
    let impulse_area_bits = if i.compare(&epsilon) != Ordering::Greater {
        area
    } else {
        impulse
    };
    Ok(AbilityGeometry {
        attenuation_min_bits,
        attenuation_factor_bits,
        impulse_area_bits,
    })
}
fn animations(
    inputs: &AbilityInputs,
    resolver: Option<&dyn AnimationResolver>,
) -> Result<Vec<LoadedAnimationSet>> {
    let Some(resolver) = resolver else {
        return Ok(Vec::new());
    };
    let mut sets = Vec::new();
    let mut common_length = None;
    for index in 0..resolver.set_count() {
        let name = string(&inputs.fields, "idle_animation")?;
        let idle = resolver.lookup(index, &name, 0)?.ok_or_else(|| {
            Error::Unsupported(format!("unknown idle animation {name} in set{index}"))
        })?;
        let mut attacks = Vec::new();
        let mut set_length = None;
        for n in 1..=5 {
            let name = string(&inputs.fields, &format!("animation{n}"))?;
            if name.is_empty() {
                break;
            }
            let bound = resolver.lookup(index, &name, 4)?.ok_or_else(|| {
                Error::Unsupported(format!("unknown ability animation {name} in set{index}"))
            })?;
            if let Some(bits) = set_length {
                if Binary::from_f32_bits(bits)?.compare(&Binary::from_f32_bits(bound.length_bits)?)
                    != Ordering::Equal
                {
                    return Err(Error::Unsupported(
                        "native ability animations have unequal lengths".into(),
                    ));
                }
            } else {
                set_length = Some(bound.length_bits);
            }
            attacks.push(bound);
        }
        if let Some(bits) = common_length {
            let current = set_length.ok_or_else(|| {
                Error::Unsupported(
                    "native first animation set has a clip but later set is empty".into(),
                )
            })?;
            if Binary::from_f32_bits(bits)?.compare(&Binary::from_f32_bits(current)?)
                != Ordering::Equal
            {
                return Err(Error::Unsupported(
                    "native ability animation lengths differ across sets".into(),
                ));
            }
        } else if index == 0 {
            common_length = set_length;
        }
        sets.push(LoadedAnimationSet { idle, attacks });
    }
    Ok(sets)
}
/// Original473CB0 zeroes all registry entries, then reads stockpiled costs.
pub fn dense_cost_words(kinds: &[u32], authored: &[Option<u32>]) -> Result<Vec<u32>> {
    if kinds.len() != authored.len() {
        return Err(Error::Unsupported(
            "resource cost domain length mismatch".into(),
        ));
    }
    kinds
        .iter()
        .zip(authored)
        .map(|(&kind, value)| match kind {
            0 => {
                let bits = value.ok_or_else(|| missing("stockpiled cost"))?;
                Binary::from_f32_bits(bits)?;
                Ok(bits)
            }
            1 | 2 => Ok(0),
            _ => Err(Error::Unsupported("unknown native Resource type".into())),
        })
        .collect()
}
fn costs(catalog: &ActorCatalog, node: &ResolvedNode<'_>, pc: Precision) -> Result<DenseCost> {
    let domain = catalog.native_resource_registry(pc)?;
    let mut kinds = Vec::new();
    let mut amounts = Vec::new();
    let mut fields = BTreeMap::new();
    let mut source_path = None;
    for id in &domain.ids {
        let resource = catalog.resolved_definition("Resource", id, pc)?;
        let sources = resource.sources();
        let path = &sources[0].path;
        if let Some(prior) = &source_path {
            if prior != path {
                return Err(Error::Unsupported(
                    "Resource native order spans multiple unproved files".into(),
                ));
            }
        } else {
            source_path = Some(path.clone());
        }
        let Some(TypedValue::String { value: kind, .. }) = resource.field("type", pc)? else {
            return Err(missing("Resource.type"));
        };
        let (kind, amount) = match kind.to_ascii_uppercase().as_str() {
            "STOCKPILED" => {
                let value = node
                    .field(id, pc)?
                    .ok_or_else(|| missing(&format!("Cost.{id}")))?;
                let amount = value.fixed_bits()?;
                fields.insert(id.clone(), value);
                (0, Some(amount))
            }
            // Original473CB0 zero-initializes the entire dense vector through45F180,
            // then reads only ResourceData.type+18==0. These zeros are native writes.
            "UPKEEP" => (1, None),
            "LIMITED_CONSUMED" => (2, None),
            _ => return Err(Error::Unsupported(format!("unknown Resource kind {kind}"))),
        };
        kinds.push(kind);
        amounts.push(amount);
    }
    Ok(DenseCost {
        resource_ids: domain.ids,
        bits: dense_cost_words(&kinds, &amounts)?,
        fields,
        resource_sources: domain.declarations,
    })
}
/// CRT `iswspace` (96CC0B) for the code units the native tokenizer
/// classifies. Units below 0x100 use the CRT table (pinned by the leading-unit
/// sweep in native-special-ops-loading-fixtures.json: 0x09..=0x0D, 0x20,
/// 0xA0). Larger units go through the OS (GetStringTypeW), which is not part
/// of the executable and is rejected rather than approximated.
fn native_iswspace(unit: u16) -> Result<bool> {
    match unit {
        0x09..=0x0D | 0x20 | 0xA0 => Ok(true),
        0..=0xFF => Ok(false),
        _ => Err(Error::Unsupported(format!(
            "iswspace of U+{unit:04X} at a token start uses GetStringTypeW (not ported)"
        ))),
    }
}
/// One 412480 walk over a 424FA0(text, "," (A059D0), flags 0) tokenizer:
/// skip iswspace units, then ',' delimiters (whitespace after a delimiter is
/// not skipped again); a '"' or '\'' starts a quoted token that ends at the
/// same quote (unterminated: the rest of the text), any other unit starts a
/// plain token that ends at ','. 730520 steps past the closing quote or the
/// terminating comma. Returns the token (empty at the end).
fn native_next_token(units: &[u16], position: &mut usize) -> Result<Vec<u16>> {
    let comma = u16::from(b',');
    if *position >= units.len() {
        return Ok(Vec::new());
    }
    let mut unit = units[*position];
    while native_iswspace(unit)? {
        *position += 1;
        if *position >= units.len() {
            return Ok(Vec::new());
        }
        unit = units[*position];
    }
    while unit == comma {
        *position += 1;
        if *position >= units.len() {
            return Ok(Vec::new());
        }
        unit = units[*position];
    }
    let at = |p: usize| units.get(p).copied().unwrap_or(0);
    let mut token = Vec::new();
    if unit == u16::from(b'"') || unit == u16::from(b'\'') {
        loop {
            *position += 1;
            let next = at(*position);
            if next == unit {
                break;
            }
            if *position >= units.len() {
                return Ok(token);
            }
            token.push(next);
        }
    } else {
        token.push(unit);
        loop {
            *position += 1;
            let next = at(*position);
            if next == comma {
                break;
            }
            if *position >= units.len() {
                return Ok(token);
            }
            token.push(next);
        }
    }
    *position += 1;
    Ok(token)
}
/// The token walk of 4BF990 (4C00D0..4C01C8, ability `required_properties`)
/// and 4C4E10 (ModifierProperty `required_properties`): tokens in authored
/// order until the first empty one. Both loops are proven identical on the
/// same inputs by native-special-ops-loading-fixtures.json.
pub fn required_property_tokens(text: &str) -> Result<Vec<String>> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let mut position = 0;
    let mut out = Vec::new();
    loop {
        let token = native_next_token(&units, &mut position)?;
        if token.is_empty() {
            return Ok(out);
        }
        out.push(String::from_utf16(&token).map_err(|_| {
            Error::Unsupported("required property token splits a surrogate pair".into())
        })?);
    }
}
/// 4FED70 after the base reader (4FEAF0): `attribute` through the 4C11E0
/// enum must satisfy 13 <= index < 25 ("Unknown multiply attribute type"),
/// STRUCTURE_COST (18) and RECRUIT_COST (23) amounts are multiplied by -1.0
/// and stored as binary32, then the stored amount must not be below -1 or
/// above 5 ("Multiply modifier out of range"; unordered passes). Returns
/// (+10 attribute, +C amount bits).
pub fn multiply_modifier(attribute: &str, amount_bits: u32, pc: Precision) -> Result<(u32, u32)> {
    let index = crate::terrain_speed::ATTRIBUTES
        .iter()
        .position(|name| name.eq_ignore_ascii_case(attribute))
        .map(|i| i as i32)
        .unwrap_or(-1);
    if !(13..25).contains(&index) {
        return Err(Error::Unsupported(format!(
            "Unknown multiply attribute type '{attribute}' loading modifiers (native throws)"
        )));
    }
    let mut amount = amount_bits;
    if index == 18 || index == 23 {
        amount = Binary::from_f32_bits(amount_bits)?
            .mul(&Binary::from_f32_bits(0xbf80_0000)?, pc)?
            .store_f32()?;
    }
    let value = Binary::from_f32_bits(amount)?;
    if value.compare(&Binary::from_f32_bits(0xbf80_0000)?) == Ordering::Less
        || value.compare(&Binary::from_f32_bits(0x40a0_0000)?) == Ordering::Greater
    {
        return Err(Error::Unsupported(format!(
            "Multiply modifier '{attribute}' out of range (native throws)"
        )));
    }
    Ok((index as u32, amount))
}
/// 5010A0: an empty `enchantment_type` leaves +C null without a lookup;
/// otherwise 48D500 on the EnchantmentType registry, unknown throws.
pub fn optional_registry_lookup<T>(
    name: &str,
    find: impl FnOnce(&str) -> Result<Option<T>>,
    what: &str,
) -> Result<Option<T>> {
    if name.is_empty() {
        return Ok(None);
    }
    find(name)?.map(Some).ok_or_else(|| {
        Error::Unsupported(format!(
            "Unknown {what} '{name}' loading modifiers (native throws)"
        ))
    })
}
/// 501150 (ability_type) / 501410 (ability_definition): always looked up
/// through 48D500 (an empty name finds nothing and throws).
pub fn required_registry_lookup<T>(
    name: &str,
    find: impl FnOnce(&str) -> Result<Option<T>>,
    what: &str,
) -> Result<T> {
    let found = if name.is_empty() { None } else { find(name)? };
    found.ok_or_else(|| {
        Error::Unsupported(format!(
            "Unknown {what} '{name}' loading modifiers (native throws)"
        ))
    })
}
/// 501150 after the ability_type lookup: the `Effect` child is required
/// (5B4410(.., 1) throws "SubNode not found."), and its `type` must be in
/// the effect enum (B3B060; "Unknown ability effect type ... loading
/// modifiers") before 4CA880 constructs it. Returns the effect kind.
pub fn ability_effect_kind(effect_type: Option<&str>) -> Result<u32> {
    let Some(name) = effect_type else {
        return Err(Error::Unsupported(
            "No effect specified for ability effect modifier: SubNode not found (native throws)"
                .into(),
        ));
    };
    crate::commander::EFFECT_KINDS
        .iter()
        .position(|kind| kind.eq_ignore_ascii_case(name))
        .map(|kind| kind as u32)
        .ok_or_else(|| {
            Error::Unsupported(format!(
                "Unknown ability effect type '{name}' loading modifiers (native throws)"
            ))
        })
}
/// 4FEEC0 after the base reader: the dense vector is sized to the Resource
/// registry ([B36530]+38) and, from the required `Production` child, every
/// resource in registry order is read with GetFixed(resource IDS) (a missing
/// field throws "Node doesn't contain field.").
pub fn resource_production(
    resource_ids: &[String],
    mut field: impl FnMut(&str) -> Result<Option<u32>>,
) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    for id in resource_ids {
        out.push(field(id)?.ok_or_else(|| {
            Error::Unsupported(format!(
                "Production node doesn't contain field '{id}' (native throws)"
            ))
        })?);
    }
    Ok(out)
}
/// What 4CC730 reads from one resolved summon object (ThingData).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SummonObjectFacts {
    /// ThingData virtual +38: 1 for ActorData (4F0500), 0 for Thing/Object
    /// data (4C9FA0).
    pub actor: bool,
    /// ActorData+174 bit 16 (+176 byte bit 0): RecruitComponent.
    pub recruit_component: bool,
    /// ActorData+184 resolved `transport_actor` and its ThingData+1C
    /// `flying` flag (0x1000); `None` when there is no transport.
    pub transport_flying: Option<bool>,
    /// ActorData+174 bit 6: PropertiesComponent.
    pub properties_component: bool,
}
/// 4CC730 checks after both objects resolved (427BD0; unknown throws first):
/// a summoned actor with a RecruitComponent and a non-flying transport
/// throws, for the object then the dynamic-campaign object; then, with a
/// `property_ids` property, both objects must be actors with a
/// PropertiesComponent (object checks before dynamic ones).
pub fn summon_resolution_checks(
    object: SummonObjectFacts,
    dynamic: SummonObjectFacts,
    property: bool,
) -> Result<()> {
    let transport = |facts: SummonObjectFacts, which: &str| -> Result<()> {
        if facts.actor && facts.recruit_component && facts.transport_flying == Some(false) {
            return Err(Error::Unsupported(format!(
                "Summon ability summons {which}actor with non-flying transport (native throws)"
            )));
        }
        Ok(())
    };
    transport(object, "")?;
    transport(dynamic, "dynamic campaign ")?;
    if property {
        for (facts, which) in [(object, ""), (dynamic, " (dynamic_campaign_object)")] {
            if !facts.actor {
                return Err(Error::Unsupported(format!(
                    "Summon ability effect property specified for non-actor{which} (native throws)"
                )));
            }
            if !facts.properties_component {
                return Err(Error::Unsupported(format!(
                    "Summon ability effect property given for actor{which} with no properties component (native throws)"
                )));
            }
        }
    }
    Ok(())
}
/// Heal type lookup (4CAB50 over the 4CC1D0 enum; ASCII case-insensitive).
/// INVALID is a registered value (0); an unregistered name throws.
pub fn heal_type(name: &str) -> Result<u32> {
    HEAL_TYPES
        .iter()
        .position(|v| v.eq_ignore_ascii_case(name))
        .map(|v| v as u32)
        .ok_or_else(|| {
            Error::Unsupported(format!(
                "Unknown IDS '{name}' specified in enum table (native throws)"
            ))
        })
}
/// ADD modifier load (4FEC70) plus the 4C4030 amount-0 rule: the attribute
/// must be an ADD attribute (index < 13 of the 4C11E0 enum), `repair` only on
/// ABILITY_HEAL (4); an ordered-zero amount is kept for AUTO_UPGRADE (11),
/// deleted (`None`) for TERRAIN_SPEED_CAP (12) and throws otherwise.
pub fn add_modifier(attribute: &str, amount_bits: u32, repair: bool) -> Result<Option<u32>> {
    let index = crate::terrain_speed::ATTRIBUTES
        .iter()
        .position(|v| v.eq_ignore_ascii_case(attribute))
        .filter(|&i| i < 13)
        .ok_or_else(|| {
            Error::Unsupported(format!(
                "Unknown add attribute type '{attribute}' loading modifiers (native throws)"
            ))
        })? as u32;
    if repair && index != 4 {
        return Err(Error::Unsupported(
            "Repair flag can only be specified for attribute add ability heal (native throws)"
                .into(),
        ));
    }
    if is_zero_bits(amount_bits) && index != 11 {
        if index == 12 {
            return Ok(None);
        }
        return Err(Error::Unsupported(
            "property: shouldn't use add 0 modifiers (native throws)".into(),
        ));
    }
    Ok(Some(index))
}
/// One FXC for the permanent-VFX search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FxcFacts {
    /// CREATE_VFX: (VFX `permanent`, FXC `duration` bits).
    pub create_vfx: Option<(bool, u32)>,
    /// Child lists checked recursively (+8 and +10).
    pub immediate: Vec<FxcFacts>,
    pub chain: Vec<FxcFacts>,
}
/// 428A30 (with 428360): some CREATE_VFX whose VFX is permanent (+24 == 1)
/// with a zero FXC duration (+64 == 0), searched recursively.
pub fn permanent_vfx_present(list: &[FxcFacts]) -> bool {
    list.iter().any(|fxc| {
        matches!(fxc.create_vfx, Some((true, duration)) if is_zero_bits(duration))
            || permanent_vfx_present(&fxc.immediate)
            || permanent_vfx_present(&fxc.chain)
    })
}
fn float_bits(value: &TypedValue, name: &str) -> Result<u32> {
    match value {
        TypedValue::Scalar(s) if s.field_type == crate::native::ScalarType::Float => Ok(s.bits),
        _ => Err(Error::Unsupported(format!("{name} must be native float"))),
    }
}
fn is_zero_bits(bits: u32) -> bool {
    bits & 0x7fff_ffff == 0
}
impl ActorCatalog {
    /// One loaded ability effect: 4BEF80 kind dispatch, 4CA880 construction
    /// and the kind's LoadData (proven by native_ability_effects.py).
    fn native_load_effect(
        &self,
        effect: &super::EffectInputs,
        node: &ResolvedNode<'_>,
        domain: &crate::targets::TargetDomain,
        pc: Precision,
    ) -> Result<LoadedEffect> {
        let kind = super::effect_kind(&string(&effect.fields, "type")?)?;
        // STEAL_RESOURCES' LoadData (4CA190) does not run the base reader:
        // its +8 filter and +10 byte keep the constructor zeros.
        let (targets, hide_from_tooltip) = if kind == 8 {
            (TargetFilter::EMPTY, false)
        } else {
            (
                domain.parse(value(&effect.fields, "targets")?, false)?,
                value(&effect.fields, "hide_from_tooltip")?.boolean()?,
            )
        };
        let data = match kind {
            0 => EffectData::Summon(Box::new(self.native_summon_effect(effect, domain, pc)?)),
            1 | 2 => EffectData::Morale {
                amount_bits: fixed(&effect.fields, "amount")?,
            },
            3 => {
                let name = string(&effect.fields, "heal_type")?;
                let heal_type = heal_type(&name)?;
                EffectData::Heal {
                    amount_bits: fixed(&effect.fields, "amount")?,
                    heal_type,
                }
            }
            4 | 5 => {
                let name = string(&effect.fields, "damage_type_IDS")?;
                if self
                    .native_named_definition("DamageType", &name, pc)?
                    .is_none()
                {
                    return Err(Error::Unsupported(format!(
                        "Unknown damage type '{name}' (native throws)"
                    )));
                }
                EffectData::Damage {
                    damage_type: identity(self, "DamageType", &name, pc)?,
                    amount_bits: fixed(&effect.fields, "amount")?,
                }
            }
            6 => {
                let child = node.child("EnchantmentProperty")?.ok_or_else(|| {
                    Error::Unsupported(
                        "required EnchantmentProperty child missing (native throws)".into(),
                    )
                })?;
                EffectData::Enchantment(Box::new(
                    self.native_enchantment_property(&child, domain, pc)?,
                ))
            }
            7 | 9 => EffectData::Plain,
            8 => {
                let child = node.child("ResourcesAmount")?.ok_or_else(|| {
                    Error::Unsupported(
                        "required ResourcesAmount child missing (native throws)".into(),
                    )
                })?;
                EffectData::StealResources(costs(self, &child, pc)?)
            }
            _ => unreachable!("effect_kind returns 0..=9"),
        };
        Ok(LoadedEffect {
            kind,
            targets,
            hide_from_tooltip,
            data,
        })
    }

    /// 4C4F90 (EnchantmentProperty LoadData) over 4C42D0 (ModifierProperty
    /// LoadData) for the inline `EnchantmentProperty` node, then the 4B4E60
    /// registration check against the shared Property registry. Modifier kinds
    /// other than FLAG and RESIST_DAMAGE are rejected, never dropped.
    fn native_enchantment_property(
        &self,
        node: &ResolvedNode<'_>,
        domain: &crate::targets::TargetDomain,
        pc: Precision,
    ) -> Result<EnchantmentProperty> {
        let required = |name: &str| {
            node.field(name, pc)?
                .ok_or_else(|| Error::Unsupported(format!("EnchantmentProperty.{name} missing")))
        };
        let TypedValue::String { value: id, .. } = required("IDS")? else {
            return Err(missing("EnchantmentProperty IDS string"));
        };
        if self.native_named_definition("Property", &id, pc)?.is_some() {
            return Err(Error::Unsupported(format!(
                "Data IDS already used: '{id}' (native 4B4E60 throws)"
            )));
        }
        let mut modifiers = Vec::new();
        for modifier in node.children("Modifier")? {
            modifiers.extend(self.native_enchantment_modifier(&modifier, domain, pc)?);
        }
        let mut metadata = BTreeMap::new();
        for name in [
            "enabled_target_flags",
            "required_properties",
            "performer_cv_bias",
            "pass_down_recruit",
            "pass_down_denizen",
            "pass_down_members",
            "is_role_property",
        ] {
            metadata.insert(name.to_owned(), required(name)?);
        }
        for name in [
            "pass_down_recruit",
            "pass_down_denizen",
            "pass_down_members",
            "is_role_property",
        ] {
            metadata[name].boolean()?;
        }
        metadata["performer_cv_bias"].fixed_bits()?;
        // 4C42D0 defers a non-empty list through 473AD0 (vtable A23C38);
        // its resolver 4C4E10 runs after loading against the full registry.
        let TypedValue::String {
            value: required_text,
            ..
        } = &metadata["required_properties"]
        else {
            return Err(missing("required_properties string"));
        };
        let required_properties =
            self.native_required_properties(required_text, &format!("for property '{id}'"), pc)?;
        let target_flags = domain.flag_word(&metadata["enabled_target_flags"])?;
        let mut fx_sources = BTreeMap::new();
        for name in ["WhenAdded", "WhenRemoved", "WhenResearched"] {
            fx_sources.insert(
                name.to_owned(),
                node.children(name)?.iter().map(|v| v.sources()).collect(),
            );
        }
        let is_technology = required("is_technology")?.boolean()?;
        let duration_bits = required("duration")?.fixed_bits()?;
        let TypedValue::String { value: kind, .. } = required("enchantment_type")? else {
            return Err(missing("enchantment_type string"));
        };
        if self
            .native_named_definition("EnchantmentType", &kind, pc)?
            .is_none()
        {
            return Err(Error::Unsupported(format!(
                "Unknown enchantment type '{kind}' reading property '{id}' (native throws)"
            )));
        }
        let enchantment_type = identity(self, "EnchantmentType", &kind, pc)?;
        let mut permanent_vfx = Vec::new();
        let when_added =
            self.native_fxc_facts(node.children("WhenAdded")?, &mut permanent_vfx, pc)?;
        for list in ["WhenRemoved", "WhenResearched"] {
            self.native_fxc_facts(node.children(list)?, &mut Vec::new(), pc)?;
        }
        if !permanent_vfx_present(&when_added) {
            return Err(Error::Unsupported(format!(
                "Enchantments require a permanent VFX on WhenAdded: '{id}' (native throws)"
            )));
        }
        Ok(EnchantmentProperty {
            id,
            sources: node.sources(),
            is_technology,
            duration_bits,
            enchantment_type,
            modifiers,
            required_properties,
            target_flags,
            metadata,
            fx_sources,
            permanent_vfx,
        })
    }

    /// The same4C4030 producer used by ordinary and enchantment properties.
    /// This forwards the unchanged complete modifier loader; it does not
    /// discard kinds unsupported by a particular runtime effect adapter.
    pub fn native_property_modifier(
        &self,
        modifier: &ResolvedNode<'_>,
        domain: &crate::targets::TargetDomain,
        pc: Precision,
    ) -> Result<Option<EnchantmentModifier>> {
        self.native_enchantment_modifier(modifier, domain, pc)
    }

    /// 4C4030: modifier type enum (4C1060 table), 4FE840 construction and the
    /// kind's LoadData, then the ADD-amount-0 rule. `None` is a modifier the
    /// native deletes after logging (ADD 0 on TERRAIN_SPEED_CAP).
    fn native_enchantment_modifier(
        &self,
        modifier: &ResolvedNode<'_>,
        domain: &crate::targets::TargetDomain,
        pc: Precision,
    ) -> Result<Option<EnchantmentModifier>> {
        use crate::property_inputs::{FlagModifierInputs, MODIFIER_FLAGS};
        let Some(TypedValue::String { value: kind, .. }) = modifier.field("type", pc)? else {
            return Err(missing("Modifier type"));
        };
        let kind = MODIFIER_TYPES
            .iter()
            .position(|v| v.eq_ignore_ascii_case(&kind))
            .ok_or_else(|| {
                Error::Unsupported(format!(
                    "Unknown modifier type '{kind}' loading modifier property (native throws)"
                ))
            })?;
        let field = |name: &str| {
            modifier
                .field(name, pc)?
                .ok_or_else(|| missing(&format!("Modifier {name}")))
        };
        let damage = |name: &str| -> Result<(DefinitionIdentity, u32)> {
            let TypedValue::String { value: id, .. } = field(name)? else {
                return Err(missing("damage type string"));
            };
            let Some(node) = self.native_named_definition("DamageType", &id, pc)? else {
                return Err(Error::Unsupported(format!(
                    "Unknown damage type '{id}' loading modifiers (native throws)"
                )));
            };
            let index = self
                .native_damage_type_registry(pc)?
                .ids
                .iter()
                .position(|s| s.eq_ignore_ascii_case(&id))
                .ok_or_else(|| {
                    Error::Unsupported("DamageType absent from verified registry".into())
                })? as u32;
            Ok((
                DefinitionIdentity {
                    kind: "DamageType".into(),
                    id,
                    sources: node.sources(),
                },
                index,
            ))
        };
        let sources = modifier.sources();
        Ok(Some(match kind {
            0 => {
                let flag = field("modifier_flag")?;
                let TypedValue::String { value: name, .. } = &flag else {
                    return Err(missing("modifier_flag string"));
                };
                let index = MODIFIER_FLAGS
                    .iter()
                    .position(|v| v.eq_ignore_ascii_case(name))
                    .ok_or_else(|| {
                        Error::Unsupported(format!("unknown native modifier flag {name}"))
                    })? as u32;
                EnchantmentModifier::Flag(FlagModifierInputs {
                    index,
                    flag,
                    sources,
                })
            }
            // 4FEC70 and the 4C4030 amount-0 rule.
            1 => {
                let amount_bits = field("amount")?.fixed_bits()?;
                let TypedValue::String { value: name, .. } = field("attribute")? else {
                    return Err(missing("attribute string"));
                };
                let repair = field("repair")?.boolean()?;
                let Some(attribute) = add_modifier(&name, amount_bits, repair)? else {
                    return Ok(None);
                };
                EnchantmentModifier::Add {
                    attribute,
                    amount_bits,
                    repair,
                    sources,
                }
            }
            5 => {
                let amount_bits = field("amount")?.fixed_bits()?;
                let (damage_type, damage_type_index) = damage("damage_type")?;
                EnchantmentModifier::ResistDamage {
                    amount_bits,
                    damage_type,
                    damage_type_index,
                    sources,
                }
            }
            // 501350.
            6 => {
                let (damage_type, damage_type_index) = damage("damage_type")?;
                EnchantmentModifier::PeriodicDamage {
                    damage_type,
                    damage_type_index,
                    amount_bits: field("amount")?.fixed_bits()?,
                    sources,
                }
            }
            // 4FED70.
            2 => {
                let amount_bits = field("amount")?.fixed_bits()?;
                let TypedValue::String { value: name, .. } = field("attribute")? else {
                    return Err(missing("attribute string"));
                };
                let (attribute, amount_bits) = multiply_modifier(&name, amount_bits, pc)?;
                EnchantmentModifier::Multiply {
                    attribute,
                    amount_bits,
                    sources,
                }
            }
            // 5010A0.
            3 => {
                let TypedValue::String { value: name, .. } = field("enchantment_type")? else {
                    return Err(missing("enchantment_type string"));
                };
                let enchantment_type = optional_registry_lookup(
                    &name,
                    |n| self.registry_identity("EnchantmentType", n, pc),
                    "enchantment type",
                )?;
                EnchantmentModifier::EnchantmentImmunity {
                    enchantment_type,
                    sources,
                }
            }
            // 501150.
            4 => {
                let TypedValue::String { value: name, .. } = field("ability_type")? else {
                    return Err(missing("ability_type string"));
                };
                let ability_type = required_registry_lookup(
                    &name,
                    |n| self.registry_identity("AbilityType", n, pc),
                    "ability type",
                )?;
                let child = modifier.child("Effect")?;
                let effect_type = match &child {
                    Some(node) => match node.field("type", pc)? {
                        Some(TypedValue::String { value, .. }) => Some(value),
                        _ => return Err(missing("Effect type string")),
                    },
                    None => None,
                };
                ability_effect_kind(effect_type.as_deref())?;
                let child = child.ok_or_else(|| missing("Effect"))?;
                let inputs = super::effect_inputs(&child, pc)?;
                EnchantmentModifier::AbilityEffect {
                    ability_type,
                    effect: Box::new(self.native_load_effect(&inputs, &child, domain, pc)?),
                    sources,
                }
            }
            // 4FEE90.
            7 => EnchantmentModifier::PeriodicHeal {
                amount_bits: field("amount")?.fixed_bits()?,
                sources,
            },
            // 501410.
            8 => {
                let TypedValue::String { value: name, .. } = field("ability_definition")? else {
                    return Err(missing("ability_definition string"));
                };
                EnchantmentModifier::ReverseAbility {
                    ability: required_registry_lookup(
                        &name,
                        |n| self.registry_identity("AbilityDefinition", n, pc),
                        "ability definition",
                    )?,
                    sources,
                }
            }
            // 4FEEC0.
            9 => {
                let resource_ids = self.native_resource_registry(pc)?.ids;
                let production = modifier.child("Production")?.ok_or_else(|| {
                    Error::Unsupported(
                        "RESOURCE_PRODUCTION Production: SubNode not found (native throws)".into(),
                    )
                })?;
                let bits = resource_production(&resource_ids, |id| {
                    production
                        .field(id, pc)?
                        .map(|v| v.fixed_bits())
                        .transpose()
                })?;
                EnchantmentModifier::ResourceProduction {
                    resource_ids,
                    bits,
                    sources,
                }
            }
            _ => unreachable!("MODIFIER_TYPES has ten entries"),
        }))
    }

    /// 48D500 on a named registry (empty -> null, translations applied),
    /// returned as the found definition's identity.
    fn registry_identity(
        &self,
        kind: &str,
        name: &str,
        pc: Precision,
    ) -> Result<Option<DefinitionIdentity>> {
        Ok(self
            .native_named_definition(kind, name, pc)?
            .map(|node| DefinitionIdentity {
                kind: kind.into(),
                id: name.to_owned(),
                sources: node.sources(),
            }))
    }

    /// Token walk (see [`required_property_tokens`]) with each token found in
    /// the Property registry (463FA0; unknown throws "Unknown required
    /// property") and head-inserted (native list order is reverse authored).
    /// Note: 4BF990 looks up immediately while loading and 4C4E10 after all
    /// loading; both resolve here against the complete native Property
    /// registry.
    fn native_required_properties(
        &self,
        text: &str,
        context: &str,
        pc: Precision,
    ) -> Result<Vec<DefinitionIdentity>> {
        let mut out = Vec::new();
        for token in required_property_tokens(text)? {
            let Some(property) = self.native_named_definition("Property", &token, pc)? else {
                return Err(Error::Unsupported(format!(
                    "Unknown required property '{token}' {context} (native throws)"
                )));
            };
            out.insert(
                0,
                DefinitionIdentity {
                    kind: "Property".into(),
                    id: token,
                    sources: property.sources(),
                },
            );
        }
        Ok(out)
    }

    /// ThingData virtual +38 by data class: ActorData (vtable A05540) returns
    /// 1 (4F0500); ObjectData (A13318) and VFXData (A10210) return 0
    /// (4C9FA0). Data classes whose vtable was not checked are rejected.
    fn native_is_actor_data(&self, id: &str) -> Result<bool> {
        let category = self.native_category(id)?;
        if category.name.starts_with("VFX") {
            return Ok(false);
        }
        match category.native_kind {
            0 | 1 => Ok(false),
            2 => Ok(true),
            other => Err(Error::Unsupported(format!(
                "ThingData virtual +38 for data kind {other} not verified"
            ))),
        }
    }
    /// 4BDB70: 427BD0 on the Thing manager ([B36530]+C, translations
    /// applied), accepted only when virtual +48 (VFX data: 1 in VFXData
    /// A10210, 0 in ActorData/ObjectData) holds. Returns the canonical IDS.
    fn native_vfx_find(&self, id: &str) -> Result<Option<String>> {
        let Some(found) = (!id.is_empty()).then(|| self.find(id)).flatten() else {
            return Ok(None);
        };
        let target = found.id.clone();
        Ok(self
            .native_category(&target)?
            .name
            .starts_with("VFX")
            .then_some(target))
    }
    /// 427BD0 on the Thing manager plus the facts 4CC730 reads.
    fn native_summon_object(
        &self,
        id: &str,
        what: &str,
        pc: Precision,
    ) -> Result<(DefinitionIdentity, SummonObjectFacts)> {
        let Some(found) = (!id.is_empty()).then(|| self.find(id)).flatten() else {
            return Err(Error::Unsupported(format!(
                "Unknown {what}'{id}' for summon ability effect (native throws)"
            )));
        };
        let target = found.id.clone();
        let actor = self.native_is_actor_data(&target)?;
        let mut facts = SummonObjectFacts {
            actor,
            recruit_component: false,
            transport_flying: None,
            properties_component: false,
        };
        if actor {
            let present = |name: &str| -> Result<bool> {
                Ok(!self.native_node_sources(&target, &[name])?.is_empty())
            };
            facts.recruit_component = present("RecruitComponent")?;
            facts.properties_component = present("PropertiesComponent")?;
            // ActorData LoadData (464CA7) reads `transport_actor` for every
            // actor; its deferred resolution 4608C0 fills +184.
            let authored = self
                .resolved_actor(&target)?
                .plain_string_field("transport_actor")?
                .ok_or_else(|| missing("ActorData transport_actor"))?;
            if let TypedValue::String { value, .. } = authored
                && !value.is_empty()
            {
                let transport = self
                    .find(&value)
                    .ok_or_else(|| {
                        Error::Unsupported(format!(
                            "Unknown actor '{value}' listed as transport actor (native throws)"
                        ))
                    })?
                    .id
                    .clone();
                facts.transport_flying =
                    Some(self.native_final_flags(&transport, pc)? & 0x1000 != 0);
            }
        }
        let node = self.resolved_actor(&target)?;
        Ok((
            DefinitionIdentity {
                kind: "Thing".into(),
                id: target,
                sources: node.sources(),
            },
            facts,
        ))
    }

    /// 4CC890 (targets/hide read by the caller's base fields) and the
    /// deferred 4CC730 resolution, performed at once here. The native runs
    /// 4CC730 after all definitions loaded; a recruit actor with a
    /// non-flying transport depends on the deferred-list order (whether the
    /// actor's own transport_actor deferral already ran), so that case is
    /// rejected explicitly instead of choosing an order.
    fn native_summon_effect(
        &self,
        effect: &super::EffectInputs,
        domain: &crate::targets::TargetDomain,
        pc: Precision,
    ) -> Result<SummonEffectData> {
        let object_ids = string(&effect.fields, "object_ids")?;
        if object_ids.is_empty() {
            return Err(Error::Unsupported(
                "Object ids missing for summon ability effect (native throws)".into(),
            ));
        }
        let dynamic_ids = string(&effect.fields, "dynamic_campaign_object_ids")?;
        let property_ids = string(&effect.fields, "property_ids")?;
        let property = if property_ids.is_empty() {
            None
        } else {
            Some(self.registry_identity("Property", &property_ids, pc)?.ok_or_else(|| {
                Error::Unsupported(format!(
                    "Property '{property_ids}' not found summon ability effect (native throws)"
                ))
            })?)
        };
        let controllable = value(&effect.fields, "controllable")?.boolean()?;
        let unique_targets = domain.parse(value(&effect.fields, "unique_target_flags")?, false)?;
        let (object, object_facts) = self.native_summon_object(&object_ids, "object ", pc)?;
        let (dynamic_object, dynamic_facts) = if dynamic_ids.is_empty() {
            (object.clone(), object_facts)
        } else {
            self.native_summon_object(&dynamic_ids, "dynamic campaign object ", pc)?
        };
        for facts in [object_facts, dynamic_facts] {
            if facts.actor && facts.recruit_component && facts.transport_flying == Some(false) {
                return Err(Error::Unsupported(
                    "summoned recruit actor with a non-flying transport: 4CC730 outcome depends on the deferred-list order (not ported)"
                        .into(),
                ));
            }
        }
        summon_resolution_checks(object_facts, dynamic_facts, property.is_some())?;
        Ok(SummonEffectData {
            object,
            dynamic_object,
            object_facts,
            dynamic_facts,
            property,
            controllable,
            unique_targets,
        })
    }

    /// FXC facts for 428A30/428360 from authored FXC nodes (42B990 type enum,
    /// 42C5E0 CREATE_VFX: VFX lookup "Could not find vfx data", cutscene
    /// rejection, +64 = duration != 0; 494F30 VFX +24 == 1 iff `permanent`).
    /// Satisfying CREATE_VFX identities are collected depth-first in authored
    /// order. Validation of other FXC kinds (PLAY_SFX audio etc.) belongs to
    /// the FX loader and is not performed here.
    fn native_fxc_facts(
        &self,
        fxcs: Vec<ResolvedNode<'_>>,
        found: &mut Vec<DefinitionIdentity>,
        pc: Precision,
    ) -> Result<Vec<FxcFacts>> {
        let mut out = Vec::new();
        for fxc in fxcs {
            let Some(TypedValue::String { value: kind, .. }) = fxc.field("type", pc)? else {
                return Err(missing("FXC type"));
            };
            let kind = FXC_TYPES
                .iter()
                .position(|v| v.eq_ignore_ascii_case(&kind))
                .ok_or_else(|| {
                    Error::Unsupported(format!("Unknown FXC type '{kind}' (native throws)"))
                })?;
            let mut create_vfx = None;
            if kind == 0 {
                let Some(TypedValue::String { value: vfx_id, .. }) = fxc.field("VFX_IDS", pc)?
                else {
                    return Err(missing("FXC VFX_IDS"));
                };
                let Some(vfx_target) = self.native_vfx_find(&vfx_id)? else {
                    return Err(Error::Unsupported(format!(
                        "Could not find vfx data: {vfx_id} (native throws)"
                    )));
                };
                let vfx = self.resolved_actor(&vfx_target)?;
                let flag = |name: &str| -> Result<bool> {
                    vfx.field(name, pc)?
                        .ok_or_else(|| missing(&format!("VFX {name}")))?
                        .boolean()
                };
                if flag("cutscene")? {
                    return Err(Error::Unsupported(format!(
                        "Cannot create cutscene VFX '{vfx_id}' using FX system (native throws)"
                    )));
                }
                let duration_bits = float_bits(
                    &fxc.field("duration", pc)?
                        .ok_or_else(|| missing("FXC duration"))?,
                    "FXC duration",
                )?;
                let permanent = flag("permanent")?;
                if permanent && is_zero_bits(duration_bits) {
                    found.push(DefinitionIdentity {
                        kind: "VFX".into(),
                        id: vfx_id,
                        sources: vfx.sources(),
                    });
                }
                create_vfx = Some((permanent, duration_bits));
            }
            out.push(FxcFacts {
                create_vfx,
                immediate: self.native_fxc_facts(fxc.children("ChainImmediate")?, found, pc)?,
                chain: self.native_fxc_facts(fxc.children("Chain")?, found, pc)?,
            });
        }
        Ok(out)
    }

    /// 4BF7B8..4BF935 for one Ability: each Effect through 4BEF80, then 4BEE20
    /// (flags bits 11+kind, main power) and the ENCHANTMENT flag check.
    /// Returns (flags, main power bits, effects).
    fn native_definition_effects(
        &self,
        input: &AbilityInputs,
        node: &ResolvedNode<'_>,
        flags: u32,
        domain: &crate::targets::TargetDomain,
        pc: Precision,
    ) -> Result<(u32, u32, Vec<LoadedEffect>)> {
        let definition_node = match &input.shared_definition_id {
            None => node
                .child("AbilityDefinition")?
                .ok_or_else(|| missing("local AbilityDefinition"))?,
            Some(TypedValue::String { value, .. }) => {
                self.resolved_definition("AbilityDefinition", value, pc)?
            }
            Some(_) => return Err(missing("ability_IDS string")),
        };
        let effect_nodes = definition_node.children("Effect")?;
        if effect_nodes.len() != input.definition.effects.len() {
            return Err(Error::Unsupported("Effect node list changed".into()));
        }
        let mut effects = Vec::new();
        for (effect, effect_node) in input.definition.effects.iter().zip(effect_nodes) {
            effects.push(self.native_load_effect(effect, &effect_node, domain, pc)?);
        }
        let (flags, power_bits) = effect_flags_and_power(
            flags,
            &effects
                .iter()
                .map(|e| match &e.data {
                    EffectData::Damage { amount_bits, .. } => (e.kind, *amount_bits),
                    _ => (e.kind, 0),
                })
                .collect::<Vec<_>>(),
        )?;
        // 4BF913..4BF935: ENCHANTMENT flag (bit 2) requires an enchantment
        // effect (bit 11 + kind 6).
        if flags & (1 << 2) != 0 && flags & (1 << 17) == 0 {
            return Err(Error::Unsupported(
                "Ability has the ENCHANTMENT flag but does not have an Enchantment ability effect (native throws)".into(),
            ));
        }
        Ok((flags, power_bits, effects))
    }

    /// The effect loading of every Performer then Defender Ability of an actor
    /// (definition flags, effects, main power), independent of the Character
    /// animation binding that `native_loaded_performer` also requires.
    pub fn native_ability_effects(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<Vec<(u32, u32, Vec<LoadedEffect>)>> {
        let domain = self.native_target_domain(pc)?;
        let actor = self.resolved_actor(id)?;
        let mut out = Vec::new();
        for component in ["PerformerComponent", "DefenderComponent"] {
            let Some(list) = actor.child(component)? else {
                continue;
            };
            for node in list.children("Ability")? {
                let input = self.native_ability_inputs(&node, pc)?;
                let mut flags = 0;
                for (i, name) in DEFINITION_FLAGS.into_iter().enumerate() {
                    if value(&input.definition.flags, name)?.boolean()? {
                        flags |= 1 << i;
                    }
                }
                out.push(self.native_definition_effects(&input, &node, flags, &domain, pc)?);
            }
        }
        Ok(out)
    }
    /// Special operations: a Faction's ordered `Ability` children (FactionData
    /// 48BC25 calls 4BF990 with actor 0), each through the same ability input
    /// reader and effect loading as actor abilities. Returns (flags, main
    /// power bits, effects) per ability in authored order.
    pub fn native_faction_ability_effects(
        &self,
        faction: &str,
        pc: Precision,
    ) -> Result<Vec<(u32, u32, Vec<LoadedEffect>)>> {
        let domain = self.native_target_domain(pc)?;
        let node = self.resolved_definition("Faction", faction, pc)?;
        let mut out = Vec::new();
        for ability in node.children("Ability")? {
            let input = self.native_ability_inputs(&ability, pc)?;
            let mut flags = 0;
            for (i, name) in DEFINITION_FLAGS.into_iter().enumerate() {
                if value(&input.definition.flags, name)?.boolean()? {
                    flags |= 1 << i;
                }
            }
            out.push(self.native_definition_effects(&input, &ability, flags, &domain, pc)?);
        }
        Ok(out)
    }
    /// Materializes the bounded ground simulation inputs before deriving count.
    /// Rendering FX/icon resources and projectile simulation are not implemented
    /// by this loader. It validates referenced source definitions, preserving IDs
    /// instead of forging pointer values or unknown native registry ordinals.
    pub fn native_loaded_performer(
        &self,
        id: &str,
        pc: Precision,
        resolver: Option<&dyn AnimationResolver>,
    ) -> Result<Option<LoadedPerformer>> {
        let Some(authored) = self.native_performer_inputs(id, pc)? else {
            return Ok(None);
        };
        let actor = self.resolved_actor(id)?;
        let has_character = actor.child("CharacterComponent")?.is_some();
        if has_character && resolver.is_none() {
            return Err(Error::Unsupported(
                "Character Ability requires original animation metadata".into(),
            ));
        }
        if !has_character && resolver.is_some() {
            return Err(Error::Unsupported(
                "non-Character Ability must not receive invented animation sets".into(),
            ));
        }
        let nodes = actor
            .child("PerformerComponent")?
            .ok_or_else(|| missing("PerformerComponent"))?
            .children("Ability")?;
        // 464BD0 loads the whole Turret list before any Ability.
        let turrets = self.native_turrets(id, pc)?;
        let abilities =
            self.native_load_abilities(&authored.abilities, nodes, resolver, &turrets, pc)?;
        Ok(Some(LoadedPerformer {
            authored,
            abilities,
        }))
    }
    /// 4BF990 for each authored local Ability of one component list, in order.
    fn native_load_abilities(
        &self,
        inputs: &[AbilityInputs],
        nodes: Vec<ResolvedNode<'_>>,
        resolver: Option<&dyn AnimationResolver>,
        turrets: &[crate::turrets::TurretDefinition],
        pc: Precision,
    ) -> Result<Vec<LoadedAbility>> {
        let domain = self.native_target_domain(pc)?;
        let mut abilities = Vec::new();
        for (input, node) in inputs.iter().zip(nodes) {
            let animation_sets = animations(input, resolver)?;
            let recharge_bits = derived_recharge(
                animation_sets
                    .first()
                    .and_then(|s| s.attacks.first())
                    .map_or(0, |a| a.length_bits),
                fixed(&input.fields, "reload_time")?,
                fixed(&input.definition.fields, "recharge_time")?,
                pc,
            )?;
            let mut flags = 0;
            for (i, name) in DEFINITION_FLAGS.into_iter().enumerate() {
                if value(&input.definition.flags, name)?.boolean()? {
                    flags |= 1 << i;
                }
            }
            let targets = domain.parse(value(&input.definition.fields, "targets")?, true)?;
            let geometry = ability_geometry(
                fixed(&input.definition.fields, "area")?,
                fixed(&input.definition.fields, "attenuation_range_min")?,
                fixed(&input.definition.fields, "attenuation_factor_max")?,
                fixed(&input.definition.fields, "impulse_area")?,
                pc,
            )?;
            let (flags, power_bits, effects) =
                self.native_definition_effects(input, &node, flags, &domain, pc)?;
            let projectile_id = string(&input.definition.fields, "projectile_IDS")?;
            let projectile = if projectile_id.is_empty() {
                None
            } else {
                if self.native_category(&projectile_id)?.registry_index != 22
                    || self
                        .resolved_actor(&projectile_id)?
                        .child("ProjectileComponent")?
                        .is_none()
                {
                    return Err(Error::Unsupported(format!(
                        "{projectile_id} is not a native projectile definition"
                    )));
                }
                Some(identity(self, "Thing", &projectile_id, pc)?)
            };
            let ability_type = string(&input.definition.fields, "ability_type")?;
            let ability_type = if ability_type.is_empty() {
                None
            } else {
                Some(identity(self, "AbilityType", &ability_type, pc)?)
            };
            let bone_id = string(&input.fields, "spawn_bone_ids")?;
            let spawn_bone = if bone_id.is_empty() {
                None
            } else {
                Some(identity(self, "Bone", &bone_id, pc)?)
            };
            // Original4C0003..4C0049: empty turret_ids stores -1; otherwise the
            // Bone lookup42C070 feeds45DD50 over the ActorData turret list,
            // which464BD0 loaded before this Ability list. -1 throws.
            let turret_ids = string(&input.fields, "turret_ids")?;
            let turret_index = if turret_ids.is_empty() {
                -1
            } else {
                let index = crate::turrets::turret_index(
                    turrets,
                    self.native_bone(&turret_ids, pc)?.as_ref(),
                );
                if index == -1 {
                    return Err(Error::Unsupported(format!(
                        "Unknown turret '{turret_ids}' referenced by actor ability (native4C0040 throws)"
                    )));
                }
                index
            };
            // 4C00AB..4C01C8: comma tokens (424FA0/412480), each resolved at
            // once in the Property registry (463FA0; unknown throws) and
            // head-inserted into AbilityData+58 (reverse authored order).
            let required_properties = self.native_required_properties(
                &string(&input.fields, "required_properties")?,
                "loading ability data",
                pc,
            )?;
            let TypedValue::Point { bits, .. } = value(&input.fields, "spawn_position")? else {
                return Err(missing("spawn_position Point3"));
            };
            let mut spawn_position_bits: [u32; 3] = bits
                .clone()
                .try_into()
                .map_err(|_| Error::Unsupported("spawn_position requires3components".into()))?;
            // Original4BF990 scales X/Y using9FDC48; native constant -1 bits are pinned in the arithmetic fixture.
            for part in &mut spawn_position_bits[..2] {
                *part = Binary::from_f32_bits(*part)?
                    .mul(&Binary::from_f32_bits(0xbf800000)?, pc)?
                    .store_f32()?;
            }
            let cost_node = node.child("Cost")?.ok_or_else(|| missing("Ability.Cost"))?;
            abilities.push(LoadedAbility {
                authored: input.clone(),
                targets,
                flags,
                power_bits,
                geometry,
                effects,
                animations: animation_sets,
                recharge_bits,
                costs: costs(self, &cost_node, pc)?,
                faction_xp_cost: value(&input.fields, "cost_faction_experience")?.integer()?,
                spawn_position_bits,
                spawn_bone,
                projectile,
                ability_type,
                turret_index,
                required_properties,
            });
        }
        if abilities.len() > u32::MAX as usize {
            return Err(Error::Unsupported(
                "Ability count exceeds native u32".into(),
            ));
        }
        Ok(abilities)
    }
    /// 464BD0 DefenderComponent: control range check, then each Ability
    /// through 4BF990 (the Performer Turret list is already loaded) and the
    /// three Defender checks (4BE720 animations, SPELL flag, no BOMBARD).
    pub fn native_loaded_defender(
        &self,
        id: &str,
        pc: Precision,
        resolver: Option<&dyn AnimationResolver>,
    ) -> Result<Option<LoadedDefender>> {
        let Some(authored) = self.native_defender_inputs(id, pc)? else {
            return Ok(None);
        };
        // 4657CA: fst +218; fcomp 0; test ah,41h; jp ok (only > 0 or NaN pass).
        let range = authored.control_range_bits;
        let nan = range & 0x7f80_0000 == 0x7f80_0000 && range & 0x7f_ffff != 0;
        if !nan && (range >> 31 != 0 || range & 0x7fff_ffff == 0) {
            return Err(Error::Unsupported(format!(
                "Defender control range must be positive in actor '{id}' (native throws)"
            )));
        }
        let actor = self.resolved_actor(id)?;
        let nodes = actor
            .child("DefenderComponent")?
            .ok_or_else(|| missing("DefenderComponent"))?
            .children("Ability")?;
        let turrets = self.native_turrets(id, pc)?;
        let mut abilities = Vec::new();
        for (input, node) in authored.abilities.iter().zip(nodes) {
            let mut loaded = self.native_load_abilities(
                std::slice::from_ref(input),
                vec![node],
                resolver,
                &turrets,
                pc,
            )?;
            let ability = loaded.pop().ok_or_else(|| missing("Defender Ability"))?;
            if ability
                .animations
                .first()
                .is_some_and(|set| !set.attacks.is_empty())
            {
                return Err(Error::Unsupported(format!(
                    "Defender abilities cannot have animations in actor '{id}' (native throws)"
                )));
            }
            if ability.flags & (1 << 1) == 0 {
                return Err(Error::Unsupported(format!(
                    "Defender abilities must have the SPELL flag in actor '{id}' (native throws)"
                )));
            }
            if ability.flags & (1 << 3) != 0 {
                return Err(Error::Unsupported(format!(
                    "Bombard abilities not allowed for defender component in actor '{id}' (native throws)"
                )));
            }
            abilities.push(ability);
        }
        Ok(Some(LoadedDefender {
            authored,
            abilities,
        }))
    }
}

#[cfg(test)]
mod original_data {
    use super::*;
    use crate::sources::{SourceLimits, V14Sources};
    use std::path::PathBuf;

    fn catalog() -> Option<ActorCatalog> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let data = root.join("../Data.rwd");
        let runtime = root.join("../reconstruction/native-game/runtime-expanded-v14");
        if !data.exists() || !runtime.exists() {
            return None;
        }
        let mut sources = V14Sources::open(
            data,
            runtime,
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .ok()?;
        ActorCatalog::load_v14(&mut sources).ok()
    }

    /// Special operations: every shipped Faction (general) loads all of its
    /// abilities through the native path, including SUMMON resolution, the
    /// round-5 modifier kinds and enchantment required_properties.
    #[test]
    fn original_special_operations_resolve() {
        let Some(catalog) = catalog() else {
            eprintln!("original Data.rwd / v14 runtime not present; skipped");
            return;
        };
        let pc = Precision::Double53;
        let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut kinds = [0usize; 10];
        let mut modifiers: BTreeMap<String, usize> = BTreeMap::new();
        let mut factions = 0;
        let mut abilities_total = 0;
        let ids: Vec<String> = catalog
            .resolved_definitions("Faction")
            .unwrap()
            .iter()
            .filter_map(|n| match n.field("IDS", pc).ok().flatten() {
                Some(TypedValue::String { value, .. }) => Some(value),
                _ => None,
            })
            .collect();
        for id in &ids {
            match catalog.native_faction_ability_effects(id, pc) {
                Ok(abilities) => {
                    factions += 1;
                    abilities_total += abilities.len();
                    for (_, _, effects) in &abilities {
                        for e in effects {
                            kinds[e.kind as usize] += 1;
                            if let EffectData::Enchantment(property) = &e.data {
                                for m in &property.modifiers {
                                    let name = format!("{m:?}");
                                    let name =
                                        name.split([' ', '(', '{']).next().unwrap().to_owned();
                                    *modifiers.entry(name).or_default() += 1;
                                }
                            }
                        }
                    }
                }
                Err(e) => failures.entry(e.to_string()).or_default().push(id.clone()),
            }
        }
        eprintln!(
            "factions loaded: {factions}/{}; abilities {abilities_total}; effect kinds {kinds:?}; modifiers {modifiers:?}",
            ids.len()
        );
        assert!(failures.is_empty(), "{failures:#?}");
        assert!(factions > 0);
    }

    /// The restoration-inventory ability groups: every original actor with a
    /// Performer or Defender list loads its ability effects; non-Character
    /// actors also complete `native_loaded_performer`.
    #[test]
    fn original_ability_effects_resolve() {
        let Some(catalog) = catalog() else {
            eprintln!("original Data.rwd / v14 runtime not present; skipped");
            return;
        };
        let pc = Precision::Double53;
        let mut failures: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut kinds = [0usize; 10];
        let mut loaded = 0;
        let ids: Vec<String> = catalog.actors.iter().map(|a| a.id.clone()).collect();
        for id in &ids {
            let Ok(actor) = catalog.resolved_actor(id) else {
                continue;
            };
            let has = |c: &str| actor.child(c).ok().flatten().is_some();
            if !has("PerformerComponent") && !has("DefenderComponent") {
                continue;
            }
            match catalog.native_ability_effects(id, pc) {
                Ok(abilities) => {
                    loaded += 1;
                    for (_, _, effects) in &abilities {
                        for e in effects {
                            kinds[e.kind as usize] += 1;
                        }
                    }
                }
                Err(e) => failures.entry(e.to_string()).or_default().push(id.clone()),
            }
            if !has("CharacterComponent")
                && has("PerformerComponent")
                && let Err(e) = catalog.native_loaded_performer(id, pc, None)
            {
                failures
                    .entry(format!("performer: {e}"))
                    .or_default()
                    .push(id.clone());
            }
        }
        eprintln!("actors with abilities loaded: {loaded}; effect kinds {kinds:?}");
        for (error, ids) in &failures {
            eprintln!("{} x {error}: {:?}", ids.len(), &ids[..ids.len().min(8)]);
        }
        for id in [
            "ger_infantry_mortar",
            "ger_bunker_artillery",
            "usa_battleship",
            "ger_engineer",
            "gbr_tank_flame",
            "rus_rocket_truck",
            "gbr_infantry_at",
            "ger_tank_destroyer",
        ] {
            catalog
                .native_ability_effects(id, pc)
                .unwrap_or_else(|e| panic!("{id}: {e}"));
        }
    }
}

#[cfg(test)]
mod native_fixtures {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../research/phase8-combat/native-ability-effect-fixtures.json"
        ))
        .unwrap()
    }
    fn names(doc: &Value, table: &str) -> Vec<String> {
        doc["enums"][table]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(i, row)| {
                assert_eq!(row[0].as_u64().unwrap() as usize, i, "{table} value order");
                row[1].as_str().unwrap().to_owned()
            })
            .collect()
    }

    #[test]
    fn enum_registrations_match_native() {
        let doc = fixture();
        assert_eq!(names(&doc, "effect"), crate::commander::EFFECT_KINDS);
        assert_eq!(names(&doc, "fxc"), FXC_TYPES);
        assert_eq!(names(&doc, "modifier"), MODIFIER_TYPES);
        assert_eq!(names(&doc, "heal"), HEAL_TYPES);
        assert_eq!(names(&doc, "attribute"), crate::terrain_speed::ATTRIBUTES);
        assert_eq!(names(&doc, "flag"), crate::property_inputs::MODIFIER_FLAGS);
    }

    #[test]
    fn effect_loaders_match_native() {
        let doc = fixture();
        let cases = doc["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 17);
        for case in cases {
            let kind = case["kind"].as_u64().unwrap() as u32;
            let words: Vec<u32> = case["words"]
                .as_array()
                .unwrap()
                .iter()
                .map(|w| w.as_u64().unwrap() as u32)
                .collect();
            let fields = &case["fields"];
            let error = case["error"].as_str();
            assert_eq!(words[1], kind, "{case}");
            assert_eq!(
                super::super::effect_kind(crate::commander::EFFECT_KINDS[kind as usize]).unwrap(),
                kind
            );
            // Base reader: +8 filter (sentinel from the recorded parse) and
            // +10 hide byte; STEAL_RESOURCES never runs it.
            let base_read = case["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|r| r.get("filter").is_some());
            assert_eq!(base_read, kind != 8, "{case}");
            if kind == 8 {
                assert_eq!(
                    (words[2], words[3], case["hide_byte"].as_u64()),
                    (0, 0, Some(0))
                );
            } else if error.is_none() {
                assert_eq!(
                    case["hide_byte"].as_u64().unwrap(),
                    fields["hide_from_tooltip"].as_u64().unwrap()
                );
            }
            match kind {
                1 | 2 => assert_eq!(u64::from(words[5]), fields["amount"].as_u64().unwrap()),
                3 => {
                    let rust = heal_type(fields["heal_type"].as_str().unwrap());
                    match error {
                        Some(_) => assert!(rust.is_err(), "{case}"),
                        None => {
                            assert_eq!(rust.unwrap(), words[6], "{case}");
                            assert_eq!(u64::from(words[5]), fields["amount"].as_u64().unwrap());
                        }
                    }
                }
                4 | 5 => {
                    // +14 damage type (DataManager find), +18 amount; an
                    // unknown name throws.
                    assert_eq!(u64::from(words[6]), fields["amount"].as_u64().unwrap());
                    assert_eq!(error.is_some(), words[5] == 0, "{case}");
                }
                6 => assert!(words[5] != 0 && error.is_none()),
                7 | 9 => assert!(error.is_none() && words[5..].iter().all(|&w| w == 0)),
                0 => assert!(error.is_some()),
                _ => {}
            }
        }
    }

    #[test]
    fn modifier_loaders_match_native() {
        let doc = fixture();
        for case in doc["modifiers"].as_array().unwrap() {
            let fields = &case["fields"];
            let error = case["error"].as_str();
            let added = case["added"].as_bool().unwrap();
            let kind = fields["type"].as_str().unwrap();
            if !kind.eq_ignore_ascii_case("ADD") {
                let known = MODIFIER_TYPES.iter().any(|t| t.eq_ignore_ascii_case(kind));
                assert_eq!(known, kind != "BOGUS", "{case}");
                continue;
            }
            let rust = add_modifier(
                fields["attribute"].as_str().unwrap(),
                fields["amount"].as_u64().unwrap() as u32,
                fields["repair"].as_u64().unwrap() != 0,
            );
            match (error, added) {
                (Some(_), _) => assert!(rust.is_err(), "{case}"),
                (None, false) => assert_eq!(rust.unwrap(), None, "{case}"),
                (None, true) => {
                    let words = case["modifier"].as_array().unwrap();
                    assert_eq!(rust.unwrap(), Some(words[4].as_u64().unwrap() as u32));
                    assert_eq!(words[2].as_u64(), Some(1));
                    assert_eq!(words[3], fields["amount"]);
                    assert_eq!(words[5], fields["repair"]);
                }
            }
        }
    }

    #[test]
    fn permanent_vfx_rules_match_native() {
        let doc = fixture();
        for case in doc["vfx"].as_array().unwrap() {
            let permanent = case["permanent"].as_u64().unwrap() != 0;
            assert_eq!(case["lifetime"].as_u64().unwrap() == 1, permanent, "{case}");
        }
        for case in doc["create_vfx"].as_array().unwrap() {
            let bits = case["duration"].as_u64().unwrap() as u32;
            assert_eq!(
                case["duration_byte"].as_u64().unwrap() != 0,
                !is_zero_bits(bits)
            );
        }
        fn facts(spec: &Value) -> Vec<FxcFacts> {
            spec.as_array()
                .unwrap()
                .iter()
                .map(|f| FxcFacts {
                    create_vfx: f["create"].as_bool().unwrap().then(|| {
                        (
                            f["lifetime"].as_u64().unwrap() == 1,
                            if f["duration_nonzero"].as_u64().unwrap() != 0 {
                                0x3f80_0000
                            } else {
                                0
                            },
                        )
                    }),
                    immediate: f.get("immediate").map(facts).unwrap_or_default(),
                    chain: f.get("chain").map(facts).unwrap_or_default(),
                })
                .collect()
        }
        for case in doc["permanent"].as_array().unwrap() {
            assert_eq!(
                permanent_vfx_present(&facts(&case["fxcs"])),
                case["result"].as_u64().unwrap() != 0,
                "{case}"
            );
        }
        for case in doc["enchantment_property"].as_array().unwrap() {
            let expect_error = case["enchantment_type"] != "ench_type_generic"
                || !case["permanent_fx"].as_bool().unwrap();
            assert_eq!(case["error"].is_string(), expect_error, "{case}");
            assert_eq!(case["duration_bits"].as_u64(), Some(0x3f80_0000));
        }
    }
}

#[cfg(test)]
mod special_ops_fixtures {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../research/phase8-combat/native-special-ops-loading-fixtures.json"
        ))
        .unwrap()
    }
    fn known(table: &[&str], name: &str) -> Result<Option<String>> {
        Ok(table
            .iter()
            .find(|k| k.eq_ignore_ascii_case(name))
            .map(|k| (*k).to_owned()))
    }

    #[test]
    fn required_property_walks_match_native() {
        let doc = fixture();
        let registry = ["tank", "infantry", "property_blitzkrieg"];
        for section in ["required", "ability_required"] {
            let cases = doc[section].as_array().unwrap();
            assert_eq!(cases.len(), 283);
            for case in cases {
                let text = case["text"].as_str().unwrap();
                // Native lookups in order, then the head-inserted list.
                let looked: Vec<&str> = case["records"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|r| r.get("property_find").and_then(Value::as_str))
                    .collect();
                let tokens = required_property_tokens(text).unwrap();
                let mut list = Vec::new();
                let mut error = false;
                let mut walked = Vec::new();
                for token in &tokens {
                    walked.push(token.clone());
                    match known(&registry, token).unwrap() {
                        Some(found) => list.insert(0, found),
                        None => {
                            error = true;
                            break;
                        }
                    }
                }
                assert_eq!(walked, looked, "{section} {text:?}");
                assert_eq!(error, !case["error"].is_null(), "{section} {text:?}");
                let native: Vec<&str> = case["list"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap())
                    .collect();
                if !error {
                    assert_eq!(list, native, "{section} {text:?}");
                }
            }
        }
    }

    #[test]
    fn non_latin_token_start_is_rejected() {
        assert!(required_property_tokens("\u{3000}tank").is_err());
        assert_eq!(
            required_property_tokens("tank\u{3000}x").unwrap(),
            vec!["tank\u{3000}x".to_owned()]
        );
    }

    fn word(case: &Value, i: usize) -> u32 {
        case["words"][i].as_u64().unwrap() as u32
    }

    #[test]
    fn modifier_kinds_match_native() {
        let doc = fixture();
        let pc = Precision::Double53;
        let enchantment = ["ench_type_generic", "ench_type_fear"];
        let ability_types = ["artillery", "null"];
        let definitions = ["gbr_supply_drop", "courage"];
        let resources = ["gold".to_owned(), "oil".to_owned(), "steel".to_owned()];
        let cases = doc["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 69);
        for case in cases {
            let fields = &case["fields"];
            let error = !case["error"].is_null();
            let kind = fields["type"].as_str().unwrap().to_ascii_uppercase();
            let kind_index = MODIFIER_TYPES.iter().position(|t| *t == kind).unwrap();
            if !error {
                assert_eq!(word(case, 2) as usize, kind_index, "{case}");
            }
            match kind.as_str() {
                "MULTIPLY" => {
                    let rust = multiply_modifier(
                        fields["attribute"].as_str().unwrap(),
                        fields["amount"].as_u64().unwrap() as u32,
                        pc,
                    );
                    match rust {
                        Err(_) => assert!(error, "{case}"),
                        Ok((attribute, amount)) => {
                            assert!(!error, "{case}");
                            assert_eq!((word(case, 4), word(case, 3)), (attribute, amount));
                        }
                    }
                }
                "ENCHANTMENT_IMMUNITY" => {
                    let rust = optional_registry_lookup(
                        fields["enchantment_type"].as_str().unwrap(),
                        |n| known(&enchantment, n),
                        "enchantment type",
                    );
                    match rust {
                        Err(_) => assert!(error, "{case}"),
                        Ok(found) => {
                            assert!(!error);
                            assert_eq!(found.is_some(), word(case, 3) != 0, "{case}");
                        }
                    }
                }
                "ABILITY_EFFECT" => {
                    let records = case["records"].as_array().unwrap();
                    let effect_type = records
                        .iter()
                        .any(|r| r.get("child").is_some() && r["present"] == true)
                        .then(|| {
                            records.iter().filter_map(|r| r.get("enum_find")).count();
                            match case["name"].as_str().unwrap() {
                                "ability_effect_morale" => "MORALE_LOSS",
                                "ability_effect_damage" => "MAIN_DAMAGE",
                                _ => "NOT_AN_EFFECT",
                            }
                        });
                    let rust = required_registry_lookup(
                        fields["ability_type"].as_str().unwrap(),
                        |n| known(&ability_types, n),
                        "ability type",
                    )
                    .and_then(|_| ability_effect_kind(effect_type));
                    match rust {
                        Err(_) => assert!(error, "{case}"),
                        Ok(effect_kind) => {
                            assert!(!error, "{case}");
                            assert!(word(case, 3) != 0);
                            assert_eq!(case["effect"][1].as_u64().unwrap() as u32, effect_kind);
                        }
                    }
                }
                "PERIODIC_HEAL" => {
                    assert!(!error);
                    assert_eq!(word(case, 3), fields["amount"].as_u64().unwrap() as u32);
                }
                "REVERSE_ABILITY" => {
                    let rust = required_registry_lookup(
                        fields["ability_definition"].as_str().unwrap(),
                        |n| known(&definitions, n),
                        "ability definition",
                    );
                    assert_eq!(rust.is_err(), error, "{case}");
                    if !error {
                        assert!(word(case, 3) != 0);
                    }
                }
                "RESOURCE_PRODUCTION" => {
                    let count = case["resource_count"].as_u64().unwrap() as usize;
                    let name = case["name"].as_str().unwrap();
                    if name == "production_missing" {
                        assert!(error);
                        continue;
                    }
                    let authored: BTreeMap<&str, u32> = match name {
                        "production_partial" => [("gold", 0x3f80_0000)].into(),
                        _ => resources
                            .iter()
                            .take(count)
                            .enumerate()
                            .map(|(i, r)| (r.as_str(), (1.5f32 * (i as f32 + 1.0)).to_bits()))
                            .collect(),
                    };
                    let rust = resource_production(&resources[..count], |id| {
                        Ok(authored.get(id).copied())
                    });
                    match rust {
                        Err(_) => assert!(error, "{case}"),
                        Ok(bits) => {
                            assert!(!error);
                            let native: Vec<u32> = case["production"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| v.as_u64().unwrap() as u32)
                                .collect();
                            assert_eq!(bits, native, "{case}");
                            assert_eq!(word(case, 4) as usize, count);
                        }
                    }
                }
                other => panic!("unexpected modifier kind {other}"),
            }
        }
    }

    #[test]
    fn summon_loading_and_resolution_match_native() {
        let doc = fixture();
        for case in doc["summons"].as_array().unwrap() {
            let fields = &case["fields"];
            let things: Vec<&Value> = case["things"].as_array().unwrap().iter().collect();
            let facts = |name: &str| -> Option<SummonObjectFacts> {
                things.iter().find(|t| t["name"] == name).map(|t| {
                    let mask = t["mask"].as_u64().unwrap();
                    SummonObjectFacts {
                        actor: t["actor"].as_bool().unwrap(),
                        recruit_component: mask & (1 << 16) != 0,
                        transport_flying: t["transport_flags"].as_u64().map(|f| f & 0x1000 != 0),
                        properties_component: mask & (1 << 6) != 0,
                    }
                })
            };
            let object_ids = fields["object_ids"].as_str().unwrap();
            let dynamic_ids = fields["dynamic_campaign_object_ids"].as_str().unwrap();
            let property_ids = fields["property_ids"].as_str().unwrap();
            // LoadData 4CC890: object_ids required, property_ids resolved now.
            let load_error = object_ids.is_empty()
                || (!property_ids.is_empty() && !property_ids.eq_ignore_ascii_case("tank"));
            assert_eq!(load_error, !case["load_error"].is_null(), "{case}");
            if load_error {
                continue;
            }
            assert_eq!(case["controllable"], fields["controllable"]);
            assert_eq!(case["deferred_vtable"], "0xa25f54");
            // Deferred 4CC730.
            let resolved = (|| -> Result<()> {
                let object = facts(object_ids).ok_or_else(|| missing("object"))?;
                let dynamic = if dynamic_ids.is_empty() {
                    object
                } else {
                    facts(dynamic_ids).ok_or_else(|| missing("dynamic"))?
                };
                summon_resolution_checks(object, dynamic, !property_ids.is_empty())
            })();
            let native_error = case["resolve_error"].as_str();
            match (&resolved, native_error) {
                (Ok(()), None) => {}
                (Err(e), Some(native)) => {
                    let key = native.split('\'').next().unwrap().trim();
                    let key = key.split("%s").next().unwrap().trim();
                    let rust = e.to_string();
                    let ok = rust.contains(key)
                        || (key.starts_with("Unknown object") && rust.contains("object"))
                        || (key.starts_with("Unknown dynamic") && rust.contains("dynamic"));
                    assert!(ok, "{rust} vs {native}");
                }
                other => panic!("{other:?} for {case}"),
            }
            if native_error.is_none() {
                let expected_dynamic = if dynamic_ids.is_empty() {
                    object_ids
                } else {
                    dynamic_ids
                };
                assert_eq!(case["resolved"]["object"], object_ids);
                assert_eq!(case["resolved"]["dynamic"], expected_dynamic);
            }
        }
    }
}

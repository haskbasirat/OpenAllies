//! Original TerrainData+38 derived movement cap. This reads ordered Property
//! references and their first ADD/TERRAIN_SPEED_CAP modifier. It is a bounded
//! source projection, not a replacement for the complete Property/FX loader.
use crate::{
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;

/// Original4C11E0 native attribute registration order.
pub const ATTRIBUTES: [&str; 25] = [
    "AV",
    "AV_MELEE",
    "AV_RANGED",
    "DV",
    "ABILITY_HEAL",
    "SUPPLY_RANGE",
    "DETECTION_RANGE",
    "MORALE",
    "MORALE_BREAKPOINT",
    "PROJECTILE_ABILITY_RANGE",
    "BOMBARD_ABILITY_RANGE",
    "AUTO_UPGRADE",
    "TERRAIN_SPEED_CAP",
    "SPEED_MAX",
    "RESUPPLY_RATE",
    "ENTRENCHMENT_RATE",
    "MORALE_RECOVERY_RATE",
    "MORALE_LOSS_RATE",
    "STRUCTURE_COST",
    "HEALTH",
    "CONTROL_RANGE",
    "ATTACK_EFFICIENCY_ACTUAL",
    "ATTACK_EFFICIENCY_DESIRED",
    "RECRUIT_COST",
    "RECHARGE_TIME",
];
#[derive(Clone, Debug, Serialize)]
pub struct ModifierProbe {
    pub kind: u32,
    pub attribute: Option<u32>,
    pub amount_bits: Option<u32>,
}
/// Whole4C3BC0 decision, preserving first matching descriptor even when its
/// amount is nonpositive. A later positive descriptor in that Property loses.
pub fn first_cap(modifiers: &[ModifierProbe]) -> Result<u32> {
    for modifier in modifiers {
        if modifier.kind == 1 {
            let index = modifier
                .attribute
                .ok_or_else(|| Error::Unsupported("ADD attribute is unresolved".into()))?;
            if index == 12 {
                return modifier
                    .amount_bits
                    .ok_or_else(|| Error::Unsupported("terrain ADD amount is unresolved".into()));
            }
        }
    }
    Ok(0)
}
/// Original4B8CB9..4B8D01. Each positive Property result replaces the prior
/// limit; mountain then overrides it. Ordered comparison ignores NaNs/zero.
pub fn finalize(native_type: u32, property_results: impl IntoIterator<Item = u32>) -> Result<u32> {
    if native_type > 2 {
        return Err(Error::Unsupported("unknown native Terrain type".into()));
    }
    let mut result = 0;
    for value in property_results {
        if f32::from_bits(value) > 0.0 {
            result = value;
        }
    }
    Ok(if native_type == 1 {
        0x38d1_b717
    } else {
        result
    })
}
#[derive(Clone, Debug, Serialize)]
pub struct Contribution {
    pub property_id: String,
    pub reference: TypedValue,
    pub sources: Vec<Provenance>,
    pub modifiers: Vec<ModifierProbe>,
    pub first_cap_bits: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct TerrainSpeed {
    pub terrain_id: String,
    pub native_type: u32,
    pub speed_limit_bits: u32,
    pub properties: Vec<Contribution>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct VariationSpeed {
    pub variation_id: String,
    pub terrain_id: String,
    pub native_type: u32,
    pub speed_limit_bits: u32,
    pub sources: Vec<Provenance>,
}
fn required(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc)?
        .ok_or_else(|| Error::Unsupported(format!("Terrain speed field {name} missing")))
}
fn string(value: &TypedValue) -> Result<&str> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(Error::Unsupported(
            "Terrain speed reference requires String".into(),
        )),
    }
}
impl ActorCatalog {
    pub fn native_terrain_speed(&self, id: &str, pc: Precision) -> Result<TerrainSpeed> {
        let terrain = self.resolved_definition("Terrain", id, pc)?;
        self.terrain_speed_node(&terrain, pc)
    }
    fn terrain_speed_node(
        &self,
        terrain: &ResolvedNode<'_>,
        pc: Precision,
    ) -> Result<TerrainSpeed> {
        let terrain_id = string(&required(terrain, "IDS", pc)?)?.to_owned();
        let kind = required(terrain, "type", pc)?;
        let native_type = match string(&kind)?.to_ascii_uppercase().as_str() {
            "LAND" => 0,
            "MOUNTAIN" => 1,
            "WATER" => 2,
            value => return Err(Error::Unsupported(format!("unknown Terrain type {value}"))),
        };
        let mut properties = Vec::new();
        for reference_node in terrain.children("Property")? {
            let reference = required(&reference_node, "property_ids", pc)?;
            let property = self.resolved_definition("Property", string(&reference)?, pc)?;
            let property_id = string(&required(&property, "IDS", pc)?)?.to_owned();
            let mut modifiers = Vec::new();
            for modifier in property.children("Modifier")? {
                let kind = required(&modifier, "type", pc)?;
                let kind = match string(&kind)?.to_ascii_uppercase().as_str() {
                    "FLAG" => 0,
                    "ADD" => 1,
                    "MULTIPLY" => 2,
                    "RESIST_DAMAGE" => 5,
                    other => {
                        return Err(Error::Unsupported(format!(
                            "terrain cap projection cannot classify modifier {other}"
                        )));
                    }
                };
                //4C3BC0 calls attribute/amount only on the ADD branch; ignored
                //modifier types keep their unrelated fields unevaluated.
                let (attribute, amount_bits) = if kind == 1 {
                    let attribute = required(&modifier, "attribute", pc)?;
                    let index = ATTRIBUTES
                        .iter()
                        .position(|value| {
                            value.eq_ignore_ascii_case(string(&attribute).unwrap_or(""))
                        })
                        .ok_or_else(|| Error::Unsupported("unknown ADD attribute".into()))?
                        as u32;
                    let amount = if index == 12 {
                        Some(required(&modifier, "amount", pc)?.fixed_bits()?)
                    } else {
                        None
                    };
                    (Some(index), amount)
                } else {
                    (None, None)
                };
                modifiers.push(ModifierProbe {
                    kind,
                    attribute,
                    amount_bits,
                });
            }
            let first_cap_bits = first_cap(&modifiers)?;
            properties.push(Contribution {
                property_id,
                reference,
                sources: property.sources(),
                modifiers,
                first_cap_bits,
            });
        }
        let speed_limit_bits = finalize(native_type, properties.iter().map(|v| v.first_cap_bits))?;
        Ok(TerrainSpeed {
            terrain_id,
            native_type,
            speed_limit_bits,
            properties,
            sources: terrain.sources(),
        })
    }
    /// Parent identity is retained for each real source variation, allowing the
    /// runtime's resolved palette to select its exact TerrainData speed field.
    pub fn native_terrain_variation_speeds(&self, pc: Precision) -> Result<Vec<VariationSpeed>> {
        let mut values = Vec::new();
        for terrain in self.resolved_definitions("Terrain")? {
            let speed = self.terrain_speed_node(&terrain, pc)?;
            for variation in terrain.children("Variation")? {
                let id = string(&required(&variation, "IDS", pc)?)?.to_owned();
                if values
                    .iter()
                    .any(|prior: &VariationSpeed| prior.variation_id.eq_ignore_ascii_case(&id))
                {
                    return Err(Error::Unsupported(format!(
                        "duplicate TerrainVariation {id}"
                    )));
                }
                let mut sources = speed.sources.clone();
                sources.extend(variation.sources());
                for property in &speed.properties {
                    sources.extend(property.sources.clone());
                }
                values.push(VariationSpeed {
                    variation_id: id,
                    terrain_id: speed.terrain_id.clone(),
                    native_type: speed.native_type,
                    speed_limit_bits: speed.speed_limit_bits,
                    sources,
                });
            }
        }
        Ok(values)
    }
}

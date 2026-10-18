//! Bounded shared Flag/ResistDamage source projection. Never drops other modifiers.
use crate::{
    abilities::DefinitionIdentity,
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
use std::collections::BTreeMap;
/// Original4C1110 registration, independently executed by native-property-flags.
pub const MODIFIER_FLAGS: [&str; 12] = [
    "TRAILBLAZING",
    "PARALYZE",
    "ENTANGLE",
    "BERSERK",
    "SUMMONED",
    "SUPPLIED",
    "INVISIBLE_TO_ENEMY",
    "REVEAL_ENEMY_RZOD",
    "MINIMAP_NO_FOG",
    "NO_UPKEEP",
    "NOT_CAPTURABLE",
    "NOT_DISPOSABLE",
];
#[derive(Clone, Debug, Serialize)]
pub struct FlagModifierInputs {
    pub index: u32,
    pub flag: TypedValue,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct SharedFlagPropertyInputs {
    pub id: String,
    pub sources: Vec<Provenance>,
    pub is_technology: TypedValue,
    /// Original4C42D0/4C4030 appends modifiers in authored order.
    pub modifiers: Vec<FlagModifierInputs>,
    /// Source values remain explicit; this is not a complete Property FX loader.
    pub metadata: BTreeMap<String, TypedValue>,
    pub fx_sources: BTreeMap<String, Vec<Vec<Provenance>>>,
}
#[derive(Clone, Debug, Serialize)]
pub enum PropertyModifierInputs {
    Flag(FlagModifierInputs),
    ResistDamage {
        amount: TypedValue,
        damage_type: DefinitionIdentity,
        damage_type_index: u32,
        sources: Vec<Provenance>,
    },
}
#[derive(Clone, Debug, Serialize)]
pub struct SharedPropertyInputs {
    pub id: String,
    pub sources: Vec<Provenance>,
    pub is_technology: TypedValue,
    pub modifiers: Vec<PropertyModifierInputs>,
    /// Native PropertyData+24. This is distinct from is_technology byte+5C.
    pub target_flags: u32,
    pub metadata: BTreeMap<String, TypedValue>,
    pub fx_sources: BTreeMap<String, Vec<Vec<Provenance>>>,
}
impl ActorCatalog {
    pub fn native_shared_flag_property(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<SharedFlagPropertyInputs> {
        let input = self.native_shared_property_inputs(id, pc)?;
        let mut modifiers = Vec::new();
        for modifier in input.modifiers {
            match modifier {
                PropertyModifierInputs::Flag(value) => modifiers.push(value),
                _ => {
                    return Err(Error::Unsupported(
                        "shared Flag property loader cannot discard RESIST_DAMAGE modifier".into(),
                    ));
                }
            }
        }
        Ok(SharedFlagPropertyInputs {
            id: input.id,
            sources: input.sources,
            is_technology: input.is_technology,
            modifiers,
            metadata: input.metadata,
            fx_sources: input.fx_sources,
        })
    }
    /// Complete supported modifier list plus source metadata. Nonempty FX and
    /// required-property dependencies still require their owning live loaders.
    pub fn native_shared_property_inputs(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<SharedPropertyInputs> {
        let node = self.resolved_definition("Property", id, pc)?;
        let required = |name: &str| {
            node.field(name, pc)?
                .ok_or_else(|| Error::Unsupported(format!("Property {id}.{name} missing")))
        };
        let TypedValue::String { value: id, .. } = required("IDS")? else {
            return Err(Error::Unsupported("Property IDS must be string".into()));
        };
        let is_technology = required("is_technology")?;
        is_technology.boolean()?;
        let mut modifiers = Vec::new();
        for modifier in node.children("Modifier")? {
            let Some(TypedValue::String { value: kind, .. }) = modifier.field("type", pc)? else {
                return Err(Error::Unsupported("Modifier type missing".into()));
            };
            if kind.eq_ignore_ascii_case("RESIST_DAMAGE") {
                let amount = modifier
                    .field("amount", pc)?
                    .ok_or_else(|| Error::Unsupported("resist amount missing".into()))?;
                amount.fixed_bits()?;
                let Some(TypedValue::String {
                    value: damage_id, ..
                }) = modifier.field("damage_type", pc)?
                else {
                    return Err(Error::Unsupported(
                        "resist damage_type missing/string required".into(),
                    ));
                };
                let damage = self.resolved_definition("DamageType", &damage_id, pc)?;
                let Some(TypedValue::String {
                    value: damage_id, ..
                }) = damage.field("IDS", pc)?
                else {
                    return Err(Error::Unsupported("DamageType IDS missing".into()));
                };
                let registry = self.native_damage_type_registry(pc)?;
                let damage_type_index = registry
                    .ids
                    .iter()
                    .position(|s| s.eq_ignore_ascii_case(&damage_id))
                    .ok_or_else(|| {
                        Error::Unsupported("DamageType absent from verified registry".into())
                    })? as u32;
                modifiers.push(PropertyModifierInputs::ResistDamage {
                    amount,
                    damage_type: DefinitionIdentity {
                        kind: "DamageType".into(),
                        id: damage_id,
                        sources: damage.sources(),
                    },
                    damage_type_index,
                    sources: modifier.sources(),
                });
                continue;
            }
            if !kind.eq_ignore_ascii_case("FLAG") {
                return Err(Error::Unsupported(format!(
                    "shared property loader cannot discard {kind} modifier"
                )));
            }
            let flag = modifier
                .field("modifier_flag", pc)?
                .ok_or_else(|| Error::Unsupported("modifier_flag missing".into()))?;
            let TypedValue::String { value: name, .. } = &flag else {
                return Err(Error::Unsupported("modifier_flag must be string".into()));
            };
            let index = MODIFIER_FLAGS
                .iter()
                .position(|v| v.eq_ignore_ascii_case(name))
                .ok_or_else(|| Error::Unsupported(format!("unknown native modifier flag {name}")))?
                as u32;
            modifiers.push(PropertyModifierInputs::Flag(FlagModifierInputs {
                index,
                flag,
                sources: modifier.sources(),
            }));
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
            metadata.insert(name.into(), required(name)?);
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
        let mut fx_sources = BTreeMap::new();
        for name in ["WhenAdded", "WhenRemoved", "WhenResearched"] {
            fx_sources.insert(
                name.into(),
                node.children(name)?.iter().map(|v| v.sources()).collect(),
            );
        }
        let target_flags = self
            .native_target_domain(pc)?
            .flag_word(&metadata["enabled_target_flags"])?;
        Ok(SharedPropertyInputs {
            id,
            sources: node.sources(),
            is_technology,
            modifiers,
            target_flags,
            metadata,
            fx_sources,
        })
    }
}

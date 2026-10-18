//! Source-bound Commander constructor inputs. Effect gameplay remains opaque.
use crate::{
    abilities::{DefinitionIdentity, effect_flags_and_power},
    actors::ActorCatalog,
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::Precision;
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct CommanderAbilityInputs {
    pub ordinal: u32,
    pub local_sources: Vec<Provenance>,
    pub shared_definition_id: Option<TypedValue>,
    pub definition: DefinitionIdentity,
    pub effect_kinds: Vec<u32>,
    pub main_power_bits: u32,
    /// Native FactionData48BD05 overwrites shared/local AbilityDefinition.range.
    pub faction_range_bits: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct CommanderConstructorInputs {
    pub faction_id: String,
    pub sources: Vec<Provenance>,
    pub abilities: Vec<CommanderAbilityInputs>,
    /// FactionData+3C `experience_max`: 48BE55 fixed getter 5B5130, then
    /// `fstp dword [edi+3C]` (the binary32 value unchanged).
    pub experience_max_bits: u32,
}
pub const EFFECT_KINDS: [&str; 10] = [
    "SUMMON",
    "MORALE_LOSS",
    "MORALE_GAIN",
    "HEAL",
    "MAIN_DAMAGE",
    "ADD_DAMAGE",
    "ENCHANTMENT",
    "CAPTURE",
    "STEAL_RESOURCES",
    "DESCRIBE_ECONOMY",
];
fn string(value: &TypedValue) -> Result<&str> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(Error::Unsupported("Commander ID must be string".into())),
    }
}
impl ActorCatalog {
    /// FactionData48BC25 enumerates ordered Ability nodes, calls4BF990(actor=0),
    /// and appends unconditionally after the load returns. This projection binds
    /// only constructor-consumed identity/power, not ability execution or FX.
    pub fn native_commander_constructor_inputs(
        &self,
        id: &str,
        pc: Precision,
    ) -> Result<CommanderConstructorInputs> {
        let node = self.resolved_definition("Faction", id, pc)?;
        let faction_id = node
            .field("IDS", pc)?
            .ok_or_else(|| Error::Unsupported("Faction IDS missing".into()))?;
        let mut abilities = Vec::new();
        for (index, local) in node.children("Ability")?.iter().enumerate() {
            let (definition, shared_definition_id) =
                if let Some(definition) = local.child("AbilityDefinition")? {
                    (definition, None)
                } else {
                    let reference = local.field("ability_IDS", pc)?.ok_or_else(|| {
                        Error::Unsupported("Faction Ability definition absent".into())
                    })?;
                    (
                        self.resolved_definition("AbilityDefinition", string(&reference)?, pc)?,
                        Some(reference),
                    )
                };
            let id = definition
                .field("IDS", pc)?
                .ok_or_else(|| Error::Unsupported("AbilityDefinition IDS missing".into()))?;
            let mut effects = Vec::new();
            for effect in definition.children("Effect")? {
                let ty = effect
                    .field("type", pc)?
                    .ok_or_else(|| Error::Unsupported("Effect type missing".into()))?;
                let kind = EFFECT_KINDS
                    .iter()
                    .position(|name| name.eq_ignore_ascii_case(string(&ty).unwrap_or("")))
                    .ok_or_else(|| Error::Unsupported("unknown native Effect kind".into()))?
                    as u32;
                let amount = if kind == 4 {
                    effect
                        .field("amount", pc)?
                        .ok_or_else(|| Error::Unsupported("main damage amount missing".into()))?
                        .fixed_bits()?
                } else {
                    0
                };
                effects.push((kind, amount));
            }
            let (_, main_power_bits) = effect_flags_and_power(0, &effects)?;
            abilities.push(CommanderAbilityInputs {
                ordinal: u32::try_from(index)
                    .map_err(|_| Error::Unsupported("Faction ability count overflow".into()))?,
                local_sources: local.sources(),
                shared_definition_id,
                definition: DefinitionIdentity {
                    kind: "AbilityDefinition".into(),
                    id: string(&id)?.to_owned(),
                    sources: definition.sources(),
                },
                effect_kinds: effects.iter().map(|x| x.0).collect(),
                main_power_bits,
                faction_range_bits: 0x7f7fffff,
            });
        }
        // Declared `int experience_max = -1` (template_faction.tgi): the int
        // field's GetFixed virtual 5B70B0 is `fild [field+10]`, then the
        // loader's fstp dword rounds the integer to binary32.
        let experience_max_bits = match node
            .field("experience_max", pc)?
            .ok_or_else(|| Error::Unsupported("Faction experience_max missing (native throws)".into()))?
        {
            TypedValue::Integer { value, .. } => aa_numeric::Binary::from_i32(value).store_f32()?,
            other => other.fixed_bits()?,
        };
        Ok(CommanderConstructorInputs {
            faction_id: string(&faction_id)?.to_owned(),
            sources: node.sources(),
            abilities,
            experience_max_bits,
        })
    }
}

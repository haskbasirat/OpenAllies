//! The selected current Fixed SVar cells, stored ONLY inside World.variables.
//! Source values initialize these actual mutable cells once during the existing
//! controlled fresh reset. Evaluations read these cells, not frozen CV cache
//! numeric snapshots. Source identities/WorldRule absence remain validated.
#[cfg(test)]
use crate::{catalog::Catalog, combat_value_source_context};
use aa_body::F32;
#[cfg(test)]
#[path = "load_combat_variables_tests.rs"]
mod tests;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Field {
    DamageMinimum,
    RangePower,
    AreaPower,
    OffenseWeight,
    DefenseWeight,
    MovementWeight,
    MovementDefault,
    RangedMinimum,
    DefenseWeightRanged,
    ModifierClampAdd,
}
pub const FIELDS: [Field; 10] = [
    Field::DamageMinimum,
    Field::RangePower,
    Field::AreaPower,
    Field::OffenseWeight,
    Field::DefenseWeight,
    Field::MovementWeight,
    Field::MovementDefault,
    Field::RangedMinimum,
    Field::DefenseWeightRanged,
    Field::ModifierClampAdd,
];
impl Field {
    pub const fn name(self) -> &'static str {
        match self {
            Self::DamageMinimum => "CombatDamageMinimum",
            Self::RangePower => "CombatValueRangePower",
            Self::AreaPower => "CombatValueAreaPower",
            Self::OffenseWeight => "CombatValueOffenseWeight",
            Self::DefenseWeight => "CombatValueDefenseWeight",
            Self::MovementWeight => "CombatValueMovementWeight",
            Self::MovementDefault => "CombatValueMovementDefault",
            Self::RangedMinimum => "CombatValueRangedMinimum",
            Self::DefenseWeightRanged => "CombatValueDefenseWeightRanged",
            Self::ModifierClampAdd => "ModifierAttributeClampAdd",
        }
    }
}
/// No public raw constructor, Clone, Default or Deserialize. This storage is a
/// borrowed view of the canonical current store, never another numeric owner.
pub struct Values<'a> {
    store: &'a crate::canonical_svars::Store,
}
impl<'a> Values<'a> {
    pub(crate) fn borrow(store: &'a crate::canonical_svars::Store) -> Self {
        Self { store }
    }
    pub fn bits(&self, field: Field) -> u32 {
        self.store.word(field.name())
    }
    /// Immutable per-call formula input projection from the current cells.
    /// No mutable state/copyback is created by this plain scalar value.
    pub fn formula_settings(&self) -> aa_combat::combat_value::Settings {
        let read = |f| F32(self.bits(f));
        aa_combat::combat_value::Settings {
            damage_minimum: read(Field::DamageMinimum),
            range_power: read(Field::RangePower),
            area_power: read(Field::AreaPower),
            offense_weight: read(Field::OffenseWeight),
            defense_weight: read(Field::DefenseWeight),
            movement_weight: read(Field::MovementWeight),
            movement_default: read(Field::MovementDefault),
        }
    }
}

#[cfg(test)]
fn clamp_source(catalog: &Catalog<'_>, pc: aa_numeric::Precision) -> Result<u32, String> {
    let name = Field::ModifierClampAdd.name();
    let registry = catalog
        .actors
        .native_data_registry("WorldRule")
        .ok_or("runtime CV WorldRule registry absent")?;
    if registry
        .entries
        .iter()
        .any(|entry| entry.ids.eq_ignore_ascii_case(name))
    {
        return Err("runtime CV ModifierAttributeClampAdd WorldRule publication is unbound".into());
    }
    let settings = catalog
        .settings
        .as_ref()
        .ok_or("runtime CV Settings owner absent")?;
    let (bits, source) = settings.unique_fixed(name, pc).map_err(|e| e.to_string())?;
    if !catalog
        .actors
        .sources
        .iter()
        .any(|s| s.path.eq_ignore_ascii_case(&source.path) && s.sha256 == source.sha256)
    {
        return Err("runtime CV modifier clamp source identity differs".into());
    }
    let mut matches = 0;
    for node in catalog
        .actors
        .resolved_definitions("SVars")
        .map_err(|e| e.to_string())?
    {
        if let Some(aa_rules::native::TypedValue::Scalar(value)) =
            node.field(name, pc).map_err(|e| e.to_string())?
            && value.source == *source
            && value.field_type == aa_rules::native::ScalarType::Fixed
            && value.bits == bits
        {
            matches += 1
        }
    }
    if matches != 1 {
        return Err(
            "runtime CV modifier clamp public value/span differs from actual registered source"
                .into(),
        );
    }
    Ok(bits)
}

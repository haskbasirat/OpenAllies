//! Actor economy source fields consumed by original464BD0 and4605B0.
//! References remain source identities until the runtime performs definition
//! binding. No PE pointer or authored file order is used as a runtime identity.
use crate::{
    abilities::dense_cost_words,
    actors::{ActorCatalog, ResolvedNode},
    native::{Error, Result, TypedValue},
    resources::Provenance,
};
use aa_numeric::{Binary, Precision};
use serde::Serialize;
use std::collections::BTreeMap;

fn required(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<TypedValue> {
    node.field(name, pc)?
        .ok_or_else(|| Error::Unsupported(format!("missing economy field {name}")))
}
fn text(value: &TypedValue) -> Result<&str> {
    match value {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(Error::Unsupported(
            "economy reference must be a string".into(),
        )),
    }
}
fn plain(node: &ResolvedNode<'_>, name: &str) -> Result<TypedValue> {
    node.plain_string_field(name)?
        .ok_or_else(|| Error::Unsupported(format!("missing economy string {name}")))
}
/// Native enum constructor462EC0 registers these exact three values.
pub fn recruitability_word(value: &str) -> Result<u32> {
    match value.to_ascii_uppercase().as_str() {
        "NO" => Ok(0),
        "YES" => Ok(1),
        "WITH_PRIMARY" => Ok(2),
        _ => Err(Error::Unsupported(format!(
            "unknown recruitable value {value}"
        ))),
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct DefinitionReference {
    pub id: String,
    pub authored: TypedValue,
    pub target_sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResourceValues {
    pub resource_ids: Vec<String>,
    pub resource_sources: Vec<Vec<Provenance>>,
    pub supply_required: TypedValue,
    pub cost_bits: Vec<u32>,
    pub upkeep_bits: Vec<u32>,
    pub production_bits: Vec<u32>,
    /// Only fields the original reads are retained. Non-stockpiled Cost entries
    /// are native zero initialization, not missing authored scalar defaults.
    pub fields: BTreeMap<String, TypedValue>,
    pub sources: Vec<Provenance>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EconomyInputs {
    pub id: String,
    pub recruitable: TypedValue,
    pub recruitable_word: u32,
    pub transport_actor: Option<DefinitionReference>,
    /// Forward source unpack_to_actor_ids. Native4605B0 resolves this and writes
    /// the inverse target+548 relation. The runtime must bind that relation from
    /// actual definition identities; this field is not the inverse relation.
    pub unpack_to_actor: Option<DefinitionReference>,
    pub resources: Option<ResourceValues>,
    pub sources: Vec<Provenance>,
}
fn reference(catalog: &ActorCatalog, value: TypedValue) -> Result<Option<DefinitionReference>> {
    let id = text(&value)?;
    if id.is_empty() {
        return Ok(None);
    }
    let target = catalog.resolved_actor(id)?;
    Ok(Some(DefinitionReference {
        id: id.to_owned(),
        authored: value,
        target_sources: target.sources(),
    }))
}
impl ActorCatalog {
    /// Source projection of the economy-specific ActorData fields. Full
    /// component-mask finalization and MobileDeployment validation belong to
    /// the definition loader, not this typed field reader.
    pub fn native_economy_inputs(&self, id: &str, pc: Precision) -> Result<EconomyInputs> {
        if self.native_category(id)?.native_kind != 2 {
            return Err(Error::Unsupported(
                "economy inputs require ActorData".into(),
            ));
        }
        let actor = self.resolved_actor(id)?;
        let recruitable = plain(&actor, "recruitable")?;
        let recruitable_word = recruitability_word(text(&recruitable)?)?;
        let transport_actor = reference(self, plain(&actor, "transport_actor")?)?;
        let unpack_to_actor = actor
            .child("MobileDeploymentComponent")?
            .map(|n| reference(self, required(&n, "unpack_to_actor_ids", pc)?))
            .transpose()?
            .flatten();
        let resources = actor
            .child("EconomyComponent")?
            .map(|n| self.economy_resource_values(&n, pc))
            .transpose()?;
        Ok(EconomyInputs {
            id: id.to_owned(),
            recruitable,
            recruitable_word,
            transport_actor,
            unpack_to_actor,
            resources,
            sources: actor.sources(),
        })
    }
    fn economy_resource_values(
        &self,
        node: &ResolvedNode<'_>,
        pc: Precision,
    ) -> Result<ResourceValues> {
        let domain = self.native_resource_registry(pc)?;
        let supply_required = required(node, "supply_required", pc)?;
        supply_required.boolean()?;
        let child = |name| {
            node.child(name)?
                .ok_or_else(|| Error::Unsupported(format!("missing EconomyComponent.{name}")))
        };
        let cost = child("Cost")?;
        let upkeep = child("Upkeep")?;
        let production = child("Production")?;
        let mut fields = BTreeMap::new();
        let mut kinds = Vec::new();
        let mut costs = Vec::new();
        let mut upkeep_bits = Vec::new();
        let mut production_bits = Vec::new();
        for id in &domain.ids {
            let definition = self.resolved_definition("Resource", id, pc)?;
            let kind = required(&definition, "type", pc)?;
            let kind = match text(&kind)?.to_ascii_uppercase().as_str() {
                "STOCKPILED" => 0,
                "UPKEEP" => 1,
                "LIMITED_CONSUMED" => 2,
                name => return Err(Error::Unsupported(format!("unknown Resource type {name}"))),
            };
            kinds.push(kind);
            costs.push(if kind == 0 {
                let value = required(&cost, id, pc)?;
                let bits = value.fixed_bits()?;
                fields.insert(format!("Cost.{id}"), value);
                Some(bits)
            } else {
                None
            });
            for (name, source, values) in [
                ("Upkeep", &upkeep, &mut upkeep_bits),
                ("Production", &production, &mut production_bits),
            ] {
                let value = required(source, id, pc)?;
                let bits = value.fixed_bits()?;
                // Original4667D1 rejects a negative upkeep or production value.
                if Binary::from_f32_bits(bits)?.compare(&Binary::from_u32(0))
                    == std::cmp::Ordering::Less
                {
                    return Err(Error::Unsupported(format!("negative {name}.{id}")));
                }
                values.push(bits);
                fields.insert(format!("{name}.{id}"), value);
            }
        }
        Ok(ResourceValues {
            resource_ids: domain.ids,
            resource_sources: domain.declarations,
            supply_required,
            cost_bits: dense_cost_words(&kinds, &costs)?,
            upkeep_bits,
            production_bits,
            fields,
            sources: node.sources(),
        })
    }
}

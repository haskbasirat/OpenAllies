//! Immutable source inputs consumed by ActorData combat-value completion.
//!
//! These are inputs, not completed +1E4/+30C/+36C values. Completion additionally
//! requires native manager order, global means, settings/UVars, CRT environment,
//! and the actual definition-load precision. A restore precision is not a
//! substitute for that source phase. No constructor marker is exposed as a CV.
use crate::catalog::Catalog;
use aa_body::F32;
use aa_combat::combat_value::AbilityLocal;
use aa_numeric::Precision;
use aa_rules::{
    abilities::LoadedAbility, actors::ResolvedNode, native::TypedValue, resources::Provenance,
};

pub type Result<T> = std::result::Result<T, String>;
fn error(value: impl std::fmt::Display) -> String {
    value.to_string()
}
fn fixed(node: &ResolvedNode<'_>, field: &str, pc: Precision) -> Result<F32> {
    node.field(field, pc)
        .map_err(error)?
        .ok_or_else(|| format!("missing CV source {field}"))?
        .fixed_bits()
        .map(F32)
        .map_err(error)
}
fn text(node: &ResolvedNode<'_>, field: &str, pc: Precision) -> Result<String> {
    match node.field(field, pc).map_err(error)? {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!("CV source {field} requires an assigned string")),
    }
}

/// Canonical ActorData references, preserving native child/list traversal order.
/// Body fields are (defense+288, health+284); `None` means its component is absent,
/// not that an unavailable source value has been silently replaced by zero.
#[derive(Clone, Debug)]
pub struct DefinitionInputs {
    pub canonical_id: String,
    pub source: Provenance,
    pub source_precision: Precision,
    pub flags_174: u32,
    pub body: Option<(F32, F32)>,
    pub speed_228: Option<F32>,
    pub performer_bias_210: Option<F32>,
    pub abilities: Vec<AbilityLocal>,
    pub properties_1d4: Vec<(String, F32)>,
    pub children_2f0: Vec<Option<String>>,
    pub weighted_35c: Vec<(String, i32)>,
}

/// Map completed4BF990/4BE9B0 fields, without recalculating their power/recharge.
/// 4BE680 reads the first attack's length in the first animation set, not its
/// use fraction or a later set. Original mapping windows are pinned separately.
pub fn ability_input(ability: &LoadedAbility) -> Result<AbilityLocal> {
    let read = |fields: &std::collections::BTreeMap<String, TypedValue>, field: &str| {
        fields
            .get(field)
            .ok_or_else(|| format!("completed Ability source {field} missing"))?
            .fixed_bits()
            .map(F32)
            .map_err(error)
    };
    Ok(AbilityLocal {
        definition: aa_combat::combat_value::AbilityDefinition {
            flags_54: ability.flags,
            range_24: read(&ability.authored.definition.fields, "range")?,
            area_38: read(&ability.authored.definition.fields, "area")?,
            projectile_3c: ability.projectile.is_some(),
            power_64: F32(ability.power_bits),
        },
        value_04: read(&ability.authored.fields, "reload_time")?,
        recharge_18: F32(ability.recharge_bits),
        first_entry_10: ability
            .animations
            .first()
            .and_then(|set| set.attacks.first())
            .map(|animation| F32(animation.length_bits)),
    })
}

/// 4BCA00 prepends successful ActorData registrations to manager+68's kind2
/// list. The native-order ActorCatalog projection retains those registrations;
/// reverse this order once, without sorting by IDS or translating identities.
#[derive(Clone, Debug)]
pub struct ActorRegistrationOrder {
    pub ids: Vec<String>,
    pub source_diagnostics: Vec<aa_rules::actors::Diagnostic>,
}
impl ActorRegistrationOrder {
    /// A conservative caller guard, not a source-completion certificate.
    /// Diagnostics include native syntax notices as well as load failures;
    /// classify them against the structured native load report before any
    /// future full-source completion claim. This binder retains every notice.
    pub fn require_no_source_diagnostics(&self) -> Result<&[String]> {
        if !self.source_diagnostics.is_empty() {
            return Err(
                "CV manager source diagnostics require classification before completion".into(),
            );
        }
        Ok(&self.ids)
    }
}
pub fn actor_manager_order(catalog: &Catalog<'_>) -> Result<ActorRegistrationOrder> {
    if catalog.actors.native_data_registry("Resource").is_none() {
        return Err("CV manager order requires the native-order definition loader".into());
    }
    let mut nodes = std::collections::BTreeMap::new();
    for tag in aa_rules::actors::native_load::THING_TAGS {
        for node in catalog.actors.resolved_definitions(tag).map_err(error)? {
            let origin = node
                .sources()
                .into_iter()
                .next()
                .ok_or("CV Thing provenance absent")?;
            let key = (
                origin.path,
                origin.sha256,
                origin.span.start,
                origin.span.end,
            );
            if nodes.insert(key, node).is_some() {
                return Err("CV Thing provenance ambiguous".into());
            }
        }
    }
    let mut ids = Vec::new();
    for definition in catalog.actors.actors.iter().rev() {
        // Registration identity must not be sent through Thing::Find again:
        // a later Translate may shadow the canonical registered name.
        let p = &definition.source;
        let node = nodes
            .get(&(p.path.clone(), p.sha256.clone(), p.span.start, p.span.end))
            .ok_or("registered CV Thing source unavailable")?;
        let category = text(node, "category", Precision::Double53)?;
        let category = aa_rules::native::CATEGORIES
            .iter()
            .find(|c| c.name.eq_ignore_ascii_case(&category))
            .ok_or("unknown registered CV category")?;
        if category.native_kind == 2 {
            ids.push(definition.id.clone());
        }
    }
    Ok(ActorRegistrationOrder {
        ids,
        source_diagnostics: catalog.actors.diagnostics.clone(),
    })
}

/// Source projection for one already canonical ActorData identity. `pc` is the
/// explicitly supplied source-load precision; this function does not certify
/// which precision the native application's definition-load thread established.
pub fn definition_inputs(
    catalog: &Catalog<'_>,
    canonical_id: &str,
    pc: Precision,
) -> Result<DefinitionInputs> {
    catalog.require_untranslated_actor_identity(canonical_id)?;
    let source = catalog
        .actors
        .find(canonical_id)
        .ok_or("canonical CV ActorData is absent")?;
    let binding = catalog.actor_bindings(canonical_id, pc)?;
    if binding.precision != pc {
        return Err("CV source precision mismatch".into());
    }
    let flags = binding
        .definition
        .component_mask
        .ok_or("completed CV component mask missing")?;
    let node = catalog.actors.resolved_actor(canonical_id).map_err(error)?;
    let body = binding
        .definition
        .ground
        .body
        .as_ref()
        .map(|value| (F32(value.defense_bits), F32(value.maximum_bits)));
    let speed = binding
        .definition
        .ground
        .mover
        .as_ref()
        .map(|value| value.speed);
    if (flags & 1 != 0) != body.is_some() || (flags & 8 != 0) != speed.is_some() {
        return Err("CV body/mover source presence disagrees with completed mask".into());
    }
    let mut abilities = Vec::new();
    let performer_bias = match &binding.performer {
        Some(performer) => {
            for ability in performer.abilities() {
                crate::saved_organization_actions::require_shared_ability_identity(
                    catalog,
                    &ability.authored.shared_definition_id,
                )
                .map_err(error)?;
                abilities.push(ability_input(ability)?);
            }
            Some(
                performer
                    .authored
                    .fields
                    .get("cv_bias")
                    .ok_or("Performer cv_bias source absent")?
                    .fixed_bits()
                    .map(F32)
                    .map_err(error)?,
            )
        }
        None => None,
    };
    if flags & (1 << 5) != 0 && performer_bias.is_none() {
        return Err("completed Performer bit has no loaded source data".into());
    }
    let mut properties = Vec::new();
    if flags & (1 << 6) != 0 {
        let data = catalog.layout_data(pc)?;
        let refs = aa_rules::native::layout_data::native_actor_properties(
            catalog.actors,
            &data,
            canonical_id,
            pc,
        )
        .map_err(error)?;
        let roots = catalog
            .actors
            .resolved_definitions("Property")
            .map_err(error)?;
        for reference in refs {
            let aa_rules::native::layout_data::PropertyRef::Registered(index) = reference else {
                return Err("CV initial Property reference unexpectedly embedded".into());
            };
            let entry = data
                .properties
                .get(index as usize)
                .ok_or("CV Property registry index missing")?;
            if entry.reloads != 0 || entry.is_technology.is_none() {
                return Err(
                    "CV Property reload/incomplete load needs its completed field owner".into(),
                );
            }
            let origin = entry
                .sources
                .first()
                .ok_or("CV Property provenance missing")?;
            let mut matching = roots
                .iter()
                .filter(|node| node.sources().first() == Some(origin));
            let definition = matching
                .next()
                .ok_or("CV completed Property source missing")?;
            if matching.next().is_some() {
                return Err("CV Property source ambiguous".into());
            }
            properties.push((
                entry.ids.clone(),
                fixed(definition, "performer_cv_bias", pc)?,
            ));
        }
        // native_actor_properties already preserves4C4CF0 head order and
        // historical reference lookup; reloaded/incomplete fields stay errors.
    }
    let mut children = Vec::new();
    if flags & (1 << 7) != 0 {
        let organization = binding
            .definition
            .ground
            .organization
            .as_ref()
            .ok_or("CV Organization source missing")?;
        let layouts = catalog.layout_data(pc)?;
        let layout = layouts
            .layouts
            .iter()
            .find(|layout| {
                layout
                    .ids
                    .eq_ignore_ascii_case(&organization.layout.definition)
            })
            .ok_or("CV canonical OrganizationLayout missing")?;
        if layout.error.is_some() {
            return Err("CV OrganizationLayout did not complete loading".into());
        }
        for &group in &layout.slot_groups {
            let child = organization
                .layout
                .parameters
                .get(group as usize)
                .ok_or("CV layout group outside parameters")?
                .clone();
            if let Some(id) = &child {
                catalog.require_untranslated_actor_identity(id)?;
            }
            children.push(child);
        }
    }
    let mut inventory: Vec<(String, String, i32)> = Vec::new();
    if flags & (1 << 10) != 0 {
        let denizen = node
            .child("DenizenComponent")
            .map_err(error)?
            .ok_or("CV Denizen source missing")?;
        for item in denizen.children("Inventory").map_err(error)? {
            let reference = text(&item, "actor_ids", pc)?;
            let target = catalog
                .actors
                .find(&reference)
                .ok_or_else(|| format!("CV Inventory reference {reference} missing"))?
                .id
                .clone();
            catalog.require_untranslated_actor_identity(&target)?;
            if catalog
                .actors
                .native_category(&target)
                .map_err(error)?
                .native_kind
                != 2
            {
                return Err("CV Inventory target is not ActorData".into());
            }
            let count = item
                .field("count", pc)
                .map_err(error)?
                .ok_or("CV Inventory count missing")?
                .integer()
                .map_err(error)?;
            // Source Inventory grouping is independent of the Denizen live
            // constructor's later organization-slot/behavior classification.
            aa_rules::native::denizen::native_inventory_insert(
                &mut inventory,
                (reference, target, count),
                |entry| &entry.0,
            );
        }
    }
    Ok(DefinitionInputs {
        canonical_id: source.id.clone(),
        source: source.source.clone(),
        source_precision: pc,
        flags_174: flags,
        body,
        speed_228: speed,
        performer_bias_210: performer_bias,
        abilities,
        properties_1d4: properties,
        children_2f0: children,
        weighted_35c: inventory
            .into_iter()
            .map(|(_, target, count)| (target, count))
            .collect(),
    })
}

//! Source-bound ActorData17C/3DC..3EC and native4FACA0 member deployment.
//!Source metadata is immutable; the actual mutable request is borrowed by
//!the one portable coordinator. No fresh Actor or Mind success is implied.
use crate::{catalog::Catalog, definitions::Bank};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{
    actors::ResolvedNode,
    native::{TypedValue, recruit_backrefs::NativeRecruitBackrefs},
};
use std::sync::Arc;
#[path = "member_deployment_fields.rs"]
pub mod fields;
pub use fields::{Result, World, deploy};

fn validate(catalog: &Catalog<'_>, bank: &Bank, d: &RuntimeDefinition) -> Result<Precision> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("member deployment source Bank/Catalog universe differs".into());
    }
    let pc = source.source_precision();
    catalog.require_untranslated_actor_identity(&d.proof.id)?;
    let binding = catalog.actor_bindings(&d.proof.id, pc)?;
    let actual = &binding.definition;
    if d.proof.category.native_kind != 2
        || actual.proof.id != d.proof.id
        || actual.proof.source != d.proof.source
        || actual.proof.template_chain != d.proof.template_chain
        || actual.proof.category != d.proof.category
        || actual.native_class != d.native_class
        || actual.flags != d.flags
        || actual.component_mask != d.component_mask
    {
        return Err("member deployment actual canonical ActorData source differs".into());
    }
    let token = bank
        .token("ActorDefinition", &d.proof.id)
        .ok_or("member deployment current ActorDefinition Bank identity absent")?;
    bank.require_usable(token)?;
    Ok(pc)
}
fn node<'a>(catalog: &'a Catalog<'_>, d: &RuntimeDefinition) -> Result<ResolvedNode<'a>> {
    let mut selected = None;
    for tag in ["Thing", "Unit", "Projectile", "VFX"] {
        //resolved_definitions_at: the tag's same error, else exactly its roots
        //whose first source is this one, in order (filtering every root).
        for n in catalog
            .actors
            .resolved_definitions_at(tag, &d.proof.source)
            .map_err(|e| e.to_string())?
        {
            if selected.is_some() {
                return Err("deployment source position is ambiguous".into());
            }
            selected = Some(n);
        }
    }
    let n = selected.ok_or("deployment canonical source position absent")?;
    if n.sources() != d.proof.template_chain {
        return Err("deployment source layers differ".into());
    }
    Ok(n)
}
fn point(node: &ResolvedNode<'_>, name: &str, pc: Precision) -> Result<[u32; 2]> {
    match node.field(name, pc).map_err(|e| e.to_string())? {
        Some(TypedValue::Point { bits, .. }) if bits.len() == 2 => Ok([bits[0], bits[1]]),
        _ => Err(format!("native deployment requires actual point2 {name}")),
    }
}
///Successful immutable source projection.462A81..99 constructor zeros;
///467693..A8 optional Denizen DeploymentInfo then45D460 ordered point reads.
pub struct SourceFields {
    definition: RuntimeDefinition,
    source_precision: Precision,
    enabled: bool,
    creation: [u32; 2],
    deployment: [u32; 2],
}
impl SourceFields {
    pub fn bind(
        catalog: &Catalog<'_>,
        bank: &Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        let pc = validate(catalog, bank, definition)?;
        let mut fields = Self {
            definition: definition.clone(),
            source_precision: pc,
            enabled: false,
            creation: [0; 2],
            deployment: [0; 2],
        };
        if let Some(denizen) = node(catalog, definition)?
            .child("DenizenComponent")
            .map_err(|e| e.to_string())?
        {
            if let Some(info) = denizen.child("DeploymentInfo").map_err(|e| e.to_string())? {
                fields.creation = point(&info, "creation_offset", pc)?;
                fields.deployment = point(&info, "deployment_offset", pc)?;
                fields.enabled = true;
            }
        }
        Ok(fields)
    }
    pub fn require_in_bank(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        if validate(catalog, bank, &self.definition)? != self.source_precision {
            return Err("deployment immutable source precision differs".into());
        }
        Ok(())
    }
    pub fn enabled_3ec(&self) -> bool {
        self.enabled
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    pub fn definition(&self) -> &RuntimeDefinition {
        &self.definition
    }
    pub fn word(&self, offset: u32) -> Result<u32> {
        match offset {
            0x3dc => Ok(self.creation[0]),
            0x3e0 => Ok(self.creation[1]),
            0x3e4 => Ok(self.deployment[0]),
            0x3e8 => Ok(self.deployment[1]),
            _ => Err("unbound deployment source field".into()),
        }
    }
}
///Borrowed immutable completed17C arena. Its exact deferred native producer
///already preserves head order and duplicates; no translated relookup occurs.
pub struct RecruitingStructures {
    owner: Arc<NativeRecruitBackrefs>,
    id: String,
    definition: RuntimeDefinition,
    source_precision: Precision,
}
impl RecruitingStructures {
    pub fn bind(
        catalog: &Catalog<'_>,
        bank: &Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        let pc = validate(catalog, bank, definition)?;
        let owner = catalog.native_recruit_backrefs(pc)?;
        let entry = owner
            .definition(&definition.proof.id)
            .map_err(|e| e.to_string())?;
        if entry.sources != definition.proof.template_chain {
            return Err("recruiting structure17C canonical source layers differ".into());
        }
        Ok(Self {
            owner,
            id: definition.proof.id.clone(),
            definition: definition.clone(),
            source_precision: pc,
        })
    }
    pub fn require_in_bank(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        if validate(catalog, bank, &self.definition)? != self.source_precision
            || !Arc::ptr_eq(
                &self.owner,
                &catalog.native_recruit_backrefs(self.source_precision)?,
            )
        {
            return Err("recruiting structure17C current source owner differs".into());
        }
        Ok(())
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
    pub fn ids(&self) -> &[String] {
        &self.owner.definitions[&self.id.to_ascii_lowercase()].ids
    }
}

#[cfg(test)]
#[path = "stored_member_deployment_tests.rs"]
mod tests;

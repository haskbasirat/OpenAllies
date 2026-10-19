//!4BDB70 readonly Thing-manager VFX references, not instantiated/preloaded FX.
//!Native4BCA00 constructors retain the category descriptor at Data+18; the
//!accepted VFX vtables return true from+48 (4F0500). Data+20 remains solely
//!owned by data_pre_request::State; this handle contains no mutable count.
use crate::{catalog::Catalog, combat_value_source_context::SourceFpContext, definitions::Bank};
use aa_rules::{
    actors::{ActorCatalog, ResolvedNode},
    native::{CATEGORIES, CategoryDescriptor, TypedValue},
    resources::{Provenance, SourceIdentity},
};
type Result<T> = std::result::Result<T, String>;
#[derive(Clone)]
pub struct Effect {
    id: String,
    source: Provenance,
    layers: Vec<Provenance>,
    category: CategoryDescriptor,
    universe: Vec<SourceIdentity>,
    context: SourceFpContext,
}
///Source-position receipt for the Resource+40 loader input. This contains no
///Resource allocation, reference count, Data+20 word or loaded-state claim.
#[derive(Clone)]
pub struct Resource40Input {
    effect: Effect,
    kind: crate::resource_objects::Kind,
    ids: Vec<u16>,
    field_source: Provenance,
}
impl Resource40Input {
    pub fn kind(&self) -> crate::resource_objects::Kind {
        self.kind
    }
    pub fn ids(&self) -> &[u16] {
        &self.ids
    }
    pub fn field_source(&self) -> &Provenance {
        &self.field_source
    }
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        let current = self.effect.resource_40_input(catalog, bank)?;
        if current.kind != self.kind
            || current.ids != self.ids
            || current.field_source != self.field_source
        {
            return Err("VFX Resource40 source-position receipt changed".into());
        }
        Ok(())
    }
    ///494F30 stores floating_ui_vfx at Data+38.495F10 reads it only after a
    ///nonnull Model40 store; preserve that lazy source dependency here.
    pub fn model_floating_ui(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<bool> {
        if self.kind != crate::resource_objects::Kind::Model {
            return Err("VFX Resource40 is not ModelData".into());
        }
        self.effect.validate(catalog, bank)?;
        let source = bank.combat_source()?;
        node(catalog.actors, &self.effect.source)?
            .field("floating_ui_vfx", source.source_precision())
            .map_err(|e| e.to_string())?
            .ok_or("VFX Model source floating_ui_vfx absent")?
            .boolean()
            .map_err(|e| e.to_string())
    }
}
fn context(catalog: &Catalog<'_>, bank: &Bank) -> Result<SourceFpContext> {
    catalog.require_source_owner()?;
    let owner = bank.combat_source()?;
    owner.completed_means()?;
    if owner.source_identities() != catalog.actors.sources.as_slice() {
        return Err("VFX source Bank/universe differs".into());
    }
    Ok(owner.context().clone())
}
fn name(units: &[u16]) -> Result<String> {
    let end = units.iter().position(|&v| v == 0).unwrap_or(units.len());
    if units[..end].iter().any(|&v| v > 127) {
        return Err("427BD0 VFX name outside proved ASCII case domain".into());
    }
    String::from_utf16(&units[..end]).map_err(|e| e.to_string())
}
fn node<'a>(actors: &'a ActorCatalog, source: &Provenance) -> Result<ResolvedNode<'a>> {
    //The native Find has ALREADY translated once. Locate that exact registered
    //source position; do not call resolved_actor/native_category with its IDS.
    let mut selected = None;
    for tag in ["Thing", "Unit", "Projectile", "VFX"] {
        //Same candidates and errors as filtering resolved_definitions(tag).
        for candidate in actors
            .resolved_definitions_at(tag, source)
            .map_err(|e| e.to_string())?
        {
            if candidate.sources().first() == Some(source) {
                if selected.is_some() {
                    return Err("VFX canonical source position is ambiguous".into());
                }
                selected = Some(candidate);
            }
        }
    }
    selected.ok_or_else(|| "VFX canonical registered source position is absent".into())
}
fn fields(
    actors: &ActorCatalog,
    id: &str,
    source: &Provenance,
    pc: aa_numeric::Precision,
) -> Result<(CategoryDescriptor, Vec<Provenance>)> {
    let n = node(actors, source)?;
    match n.field("IDS", pc).map_err(|e| e.to_string())? {
        Some(TypedValue::String { value, .. }) if value.eq_ignore_ascii_case(id) => {}
        _ => return Err("VFX public canonical IDS differs from actual registered source".into()),
    }
    let Some(TypedValue::String { value, .. }) =
        n.field("category", pc).map_err(|e| e.to_string())?
    else {
        return Err("VFX category source must be native String".into());
    };
    let category = CATEGORIES
        .iter()
        .find(|v| v.name.eq_ignore_ascii_case(&value))
        .copied()
        .ok_or("VFX actual category descriptor is unbound")?;
    Ok((category, n.sources()))
}
fn vfx_virtual_48(kind: u32) -> Result<bool> {
    match kind {
        5..=17 | 22 | 23 => Ok(true),
        0 => Ok(false),
        _ => Err(format!(
            "4BDB70 non-VFX kind{kind} needs its actual constructor/vf48 proof"
        )),
    }
}
///Same native Thing lookup including its one-hop translation. A reference is
///only the actual Data identity/descriptor projection; it cannot preload,
///execute or render a VFX payload and cannot change Data+20.
pub fn find(catalog: &Catalog<'_>, bank: &Bank, ids: &[u16]) -> Result<Option<Effect>> {
    let ctx = context(catalog, bank)?;
    let query = name(ids)?;
    if query.is_empty() {
        return Ok(None);
    }
    let Some(actual) = catalog.actors.find(&query) else {
        return Ok(None);
    };
    let (category, layers) = fields(catalog.actors, &actual.id, &actual.source, ctx.precision())?;
    if !vfx_virtual_48(category.native_kind)? {
        return Ok(None);
    }
    Ok(Some(Effect {
        id: actual.id.clone(),
        source: actual.source.clone(),
        layers,
        category,
        universe: catalog.actors.sources.clone(),
        context: ctx,
    }))
}
impl Effect {
    ///A completed source graph already performed historical one-hop Find.
    ///Borrow its closed reference capability; a raw public Reference/ID cannot
    ///construct this handle and no second current translation is performed.
    pub fn from_registered_reference(
        catalog: &Catalog<'_>,
        bank: &Bank,
        registered: &crate::fx_graph::RegisteredReference<'_>,
    ) -> Result<Self> {
        registered.require_vfx_reference(catalog, bank)?;
        let reference = registered.reference();
        let ctx = context(catalog, bank)?;
        let source = reference
            .source
            .first()
            .ok_or("compiled VFX canonical source absent")?;
        let (category, layers) = fields(catalog.actors, &reference.id, source, ctx.precision())?;
        if layers != reference.source || !vfx_virtual_48(category.native_kind)? {
            return Err("compiled FXC reference is not the same registered VFX allocation".into());
        }
        let effect = Self {
            id: reference.id.clone(),
            source: source.clone(),
            layers,
            category,
            universe: catalog.actors.sources.clone(),
            context: ctx,
        };
        effect.validate(catalog, bank)?;
        Ok(effect)
    }
    ///495570 (kind11) and495F10 (kinds10/14/17) GetString then4763C0.
    ///The same canonical registered root is used, with no second translation.
    ///This is only the selected field producer, conditional on unrelated
    ///type-LoadData completion; it is not a full VFX loader/source-order proof.
    pub fn resource_40_input(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<Resource40Input> {
        self.validate(catalog, bank)?;
        let owner = bank.combat_source()?;
        let pc = owner.source_precision();
        let field = match self.category.native_kind {
            11 => "texture",
            10 | 14 | 17 => "model",
            _ => return Err("VFX constructor has no494480 Resource40 branch".into()),
        };
        //A shared Data re-Load can replace this pointer. The current resolved
        //node is not a substitute for its missing type-specific load order.
        let declarations = catalog
            .actors
            .thing_ids_declarations(&self.id, pc)
            .map_err(|e| e.to_string())?;
        if declarations != 1 {
            return Err("VFX Resource40 requires actual shared reload/field owner".into());
        }
        let Some(TypedValue::String { value, source, .. }) = node(catalog.actors, &self.source)?
            .field(field, pc)
            .map_err(|e| e.to_string())?
        else {
            return Err("VFX Resource40 requires its actual String getter".into());
        };
        let normalized = self.source.path.replace('\\', "/");
        let folder = normalized.rsplit_once('/').map_or("", |(folder, _)| folder);
        let ids = crate::data_pre_request::path::relative(folder, &value)?
            .encode_utf16()
            .collect();
        Ok(Resource40Input {
            effect: self.clone(),
            kind: if field == "texture" {
                crate::resource_objects::Kind::Texture
            } else {
                crate::resource_objects::Kind::Model
            },
            ids,
            field_source: source,
        })
    }
    pub fn id(&self) -> &str {
        &self.id
    }
    pub fn sources(&self) -> &[Provenance] {
        &self.layers
    }
    ///Typed immutable canonical identity. Repeated handles are the same native
    ///definition identity, while each list occurrence remains a separate entry.
    pub fn same_identity(&self, other: &Self) -> bool {
        self.id == other.id
            && self.source == other.source
            && self.universe == other.universe
            && self.context.precision() == other.context.precision()
            && self.context.reference_startup() == other.context.reference_startup()
    }
    pub fn validate(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<()> {
        let ctx = context(catalog, bank)?;
        if self.universe != catalog.actors.sources
            || ctx.precision() != self.context.precision()
            || ctx.reference_startup() != self.context.reference_startup()
        {
            return Err("VFX reference source Bank/context changed".into());
        }
        //No second native Find: canonical names can themselves be translation keys.
        let actual = catalog
            .actors
            .actors
            .iter()
            .filter(|a| a.id.eq_ignore_ascii_case(&self.id) && a.source == self.source)
            .collect::<Vec<_>>();
        if actual.len() != 1 {
            return Err("VFX canonical allocation/source projection changed".into());
        }
        let (category, layers) = fields(catalog.actors, &self.id, &self.source, ctx.precision())?;
        if category != self.category
            || layers != self.layers
            || !vfx_virtual_48(category.native_kind)?
        {
            return Err("VFX descriptor/source layers changed".into());
        }
        Ok(())
    }
    ///The actual stored Data+18 -> category descriptor+18 field. This remains a
    ///separate reached operation after a caller's null check, as43F8E8/43F92D.
    pub fn kind(&self, catalog: &Catalog<'_>, bank: &Bank) -> Result<u32> {
        self.validate(catalog, bank)?;
        Ok(self.category.native_kind)
    }
}

#[cfg(test)]
#[path = "view_vfx_source_tests.rs"]
mod tests;

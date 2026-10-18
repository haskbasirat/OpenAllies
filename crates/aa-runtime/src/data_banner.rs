//!Selected BannerData46D630/Plane46D370/Decal46CEE0 source fields and
//!46C7A0→46C830→46C8F0 requests. The actual Banner manager has its own
//!canonical allocations; these are not fabricated Thing Data/Bank tokens.
use super::{Result, State};
use crate::{
    catalog::Catalog,
    definitions::Bank,
    resource_depot::{Backend, Resources},
    resource_objects::{Handle, TexturePolicy},
};
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode, native::TypedValue, resources::Provenance};
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};
#[derive(Clone, Debug)]
pub(super) struct Reference {
    owner: Weak<()>,
    canonical: String,
    source: Provenance,
}
#[derive(Debug)]
pub(super) struct Directory(BTreeMap<String, Banner>);
#[path = "data_banner_fields.rs"]
mod fields;
#[derive(Debug)]
struct Banner {
    layers: Vec<Provenance>,
    core: fields::Core<Handle>,
    complete: bool,
    failed: Option<String>,
}
impl Directory {
    pub(super) fn new() -> Self {
        Self(BTreeMap::new())
    }
}
impl Banner {
    fn constructed(layers: Vec<Provenance>) -> Self {
        Self {
            layers,
            core: fields::Core::constructed(),
            complete: false,
            failed: None,
        }
    }
}
fn text(n: &ResolvedNode<'_>, key: &str, pc: Precision) -> Result<String> {
    match n.field(key, pc).map_err(|e| e.to_string())? {
        Some(TypedValue::String { value, .. }) if value.is_ascii() && !value.contains('\0') => {
            Ok(value)
        }
        _ => Err(format!("Banner {key} requires actual plain ASCII String")),
    }
}
fn flag(n: &ResolvedNode<'_>, key: &str, pc: Precision) -> Result<bool> {
    n.field(key, pc)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Banner {key} source absent"))?
        .boolean()
        .map_err(|e| e.to_string())
}
fn canonical<'a>(catalog: &'a Catalog<'_>, r: &Reference) -> Result<ResolvedNode<'a>> {
    //The native Find has already translated once; locate that registered
    //source allocation directly, never Find its canonical name again.
    let mut selected = None;
    for node in catalog
        .actors
        .resolved_definitions("Banner")
        .map_err(|e| e.to_string())?
    {
        if node.sources().first() == Some(&r.source) {
            if selected.is_some() {
                return Err("Banner canonical source is ambiguous".into());
            }
            selected = Some(node)
        }
    }
    selected.ok_or_else(|| "Banner canonical registered source is absent".into())
}
fn source_pc(catalog: &Catalog<'_>, bank: &Bank) -> Result<Precision> {
    catalog.require_source_owner()?;
    let source = bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return Err("Banner source Bank/universe differs".into());
    }
    if catalog.actors.native_data_registry("Banner").is_none() {
        return Err("Banner actual native manager registry is unavailable".into());
    }
    Ok(source.source_precision())
}
impl State {
    ///Actual Find before the ObjectData+134 pointer store. The referenced
    ///Banner allocation remains owned even when its selected loader fails.
    pub(super) fn bind_banner(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        ids: &str,
    ) -> Result<Reference> {
        let pc = source_pc(catalog, bank)?;
        if self
            .sources
            .as_ref()
            .is_some_and(|s| s != &catalog.actors.sources)
            || self.source_precision.is_some_and(|p| p != pc)
        {
            return Err("Banner DataState source universe/PC differs".into());
        }
        let node = catalog
            .actors
            .native_named_definition("Banner", ids, pc)
            .map_err(|e| e.to_string())?
            .ok_or("49E5BA unknown Banner")?;
        let name = text(&node, "IDS", pc)?;
        let layers = node.sources();
        let source = layers
            .first()
            .ok_or("Banner canonical provenance absent")?
            .clone();
        let key = name.to_ascii_lowercase();
        if let Some(old) = self.banners.0.get(&key) {
            if old.layers != layers {
                return Err("Banner canonical allocation/source changed".into());
            }
        } else {
            self.banners.0.insert(key, Banner::constructed(layers));
        }
        if self.sources.is_none() {
            self.sources = Some(catalog.actors.sources.clone());
            self.source_precision = Some(pc);
        }
        Ok(Reference {
            owner: Arc::downgrade(&self.owner),
            canonical: name,
            source,
        })
    }
    fn banner(&self, catalog: &Catalog<'_>, bank: &Bank, r: &Reference) -> Result<&Banner> {
        let pc = source_pc(catalog, bank)?;
        if r.owner.upgrade().is_none() || !Weak::ptr_eq(&r.owner, &Arc::downgrade(&self.owner)) {
            return Err("foreign or destroyed Banner allocation owner".into());
        }
        if self.sources.as_ref() != Some(&catalog.actors.sources)
            || self.source_precision != Some(pc)
        {
            return Err("Banner DataState source owner/PC changed".into());
        }
        let entry = self
            .banners
            .0
            .get(&r.canonical.to_ascii_lowercase())
            .ok_or("Banner allocation missing")?;
        let node = canonical(catalog, r)?;
        if node.sources() != entry.layers || text(&node, "IDS", pc)? != r.canonical {
            return Err("Banner canonical source proof changed".into());
        }
        Ok(entry)
    }
    pub(super) fn prepare_banner(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        r: &Reference,
        resources: &mut Resources<'_>,
        policy: &mut impl TexturePolicy,
    ) -> Result<()> {
        let pc = source_pc(catalog, bank)?;
        let old = self.banner(catalog, bank, r)?;
        if let Some(error) = &old.failed {
            return Err(error.clone());
        }
        if old.complete {
            return Ok(());
        }
        let root = canonical(catalog, r)?;
        let normalized = r.source.path.replace('\\', "/");
        let folder = normalized.rsplit_once('/').map_or("", |(f, _)| f);
        let out = self
            .banners
            .0
            .get_mut(&r.canonical.to_ascii_lowercase())
            .unwrap();
        let result = out.core.load(
            &root,
            &mut Current {
                catalog,
                bank,
                resources,
                policy,
                pc,
                folder,
                node: std::marker::PhantomData,
            },
        );
        if result.is_ok() {
            out.complete = true;
        }
        if let Err(error) = &result {
            out.failed = Some(error.clone());
        }
        result
    }
    pub(super) fn banner_aspect_input(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        r: &Reference,
    ) -> Result<(Vec<u16>, Option<Handle>, Vec<Provenance>, String)> {
        let source = self.banner(catalog, bank, r)?;
        if !source.complete {
            return Err(source
                .failed
                .clone()
                .unwrap_or_else(|| "Banner selected source fields unfinished".into()));
        }
        let path = r.source.path.replace('\\', "/");
        let folder = path.rsplit_once('/').map_or("", |(f, _)| f).to_string();
        Ok((
            source.core.model.encode_utf16().collect(),
            source.core.skin.clone(),
            source.layers.clone(),
            folder,
        ))
    }
    pub(super) fn request_banner(
        &self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        r: &Reference,
        resources: &mut Resources<'_>,
        backend: &mut impl Backend,
        delta: u32,
        mode: u32,
    ) -> Result<()> {
        let source = self.banner(catalog, bank, r)?;
        if !source.complete {
            return Err(source
                .failed
                .clone()
                .unwrap_or_else(|| "Banner selected source fields unfinished".into()));
        }
        source.core.request(delta, mode, |handle, delta, mode| {
            resources.request_10(handle, delta as i32, mode, backend)
        })
    }
}

struct Current<'a, 'catalog, 'manager, 'node, T> {
    catalog: &'a Catalog<'catalog>,
    bank: &'a Bank,
    resources: &'a mut Resources<'manager>,
    policy: &'a mut T,
    pc: Precision,
    folder: &'a str,
    node: std::marker::PhantomData<&'node ()>,
}
impl<'node, T: TexturePolicy> fields::Source<Handle> for Current<'_, '_, '_, 'node, T> {
    type Node = ResolvedNode<'node>;
    fn text(&mut self, n: &Self::Node, key: &str) -> Result<String> {
        text(n, key, self.pc)
    }
    fn flag(&mut self, n: &Self::Node, key: &str) -> Result<bool> {
        flag(n, key, self.pc)
    }
    fn children(&mut self, n: &Self::Node, key: &str) -> Result<Vec<Self::Node>> {
        n.children(key).map_err(|e| e.to_string())
    }
    fn path(&mut self, format: &str, prefix: &str) -> Result<String> {
        super::path::relative(self.folder, &super::path::expand(format, prefix)?)
    }
    fn texture(&mut self, path: &str) -> Result<Option<Handle>> {
        self.resources.texture_5a77e0_with_policy(
            self.catalog,
            self.bank,
            &path.encode_utf16().collect::<Vec<_>>(),
            0,
            self.policy,
        )
    }
}

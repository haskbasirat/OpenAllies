//!Selected Aspect46A510/46A8A0 fields and their SAME application's469FE0
//!model owner. This does not certify completion of ObjectData::LoadData.
//!In particular a positive Banner replacement requires its actual source
//!loader and may not be represented by a chosen ordinary aspect.
use super::{Data, Result, State};
use crate::{
    catalog::Catalog,
    definitions::Bank,
    resource_depot::{Backend, Resources},
    resource_objects::{Handle, ModelPolicy},
    vworld_resource_context::{Biome, BiomeIdentity},
};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::{
    actors::ResolvedNode,
    native::{ScalarType, TypedValue},
    resources::Provenance,
};
#[path = "data_aspect_fields.rs"]
mod fields;
#[cfg(test)]
#[path = "data_aspect_tests.rs"]
mod tests;

///Readonly capability to one published Aspect in the SAME canonical Data.
#[derive(Clone, Debug)]
pub struct Aspect {
    data: Data,
    index: usize,
}
pub(super) struct Object {
    definition: RuntimeDefinition,
    rows: Vec<Row>,
    pending: Option<Row>,
    complete: bool,
}
impl std::fmt::Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectAspectFields")
            .field("rows", &self.rows.len())
            .field("pending", &self.pending.is_some())
            .field("complete", &self.complete)
            .finish()
    }
}
struct Row {
    input: Option<Input>,
    count: u32,
    biome: Option<BiomeIdentity>,
    model: Option<Handle>,
    skins: [Option<Handle>; 2],
    skin_stage: usize,
    animations: Option<crate::object_animation_set::Set>,
    animation_stage: usize,
    source_ready: bool,
    extra_count: u32,
    extra_requested: bool,
}
struct Input {
    sources: Vec<Provenance>,
    model: Vec<u16>,
    visual_radius: u32,
    skin_paths: [String; 2],
    texture_paths: Vec<String>,
    resource_prefix: String,
    folder: String,
}
fn node<'a>(catalog: &'a Catalog<'_>, d: &RuntimeDefinition) -> Result<ResolvedNode<'a>> {
    let mut selected = None;
    for tag in ["Thing", "Unit", "Projectile", "VFX"] {
        for n in catalog
            .actors
            .resolved_definitions(tag)
            .map_err(|e| e.to_string())?
        {
            if n.sources().first() == Some(&d.proof.source) {
                if selected.is_some() {
                    return Err("Aspect canonical source position is ambiguous".into());
                }
                selected = Some(n);
            }
        }
    }
    let n = selected.ok_or("Aspect canonical registered source absent")?;
    if n.sources() != d.proof.template_chain {
        return Err("Aspect canonical source layers differ".into());
    }
    Ok(n)
}
fn value(n: &ResolvedNode<'_>, key: &str, p: Precision) -> Result<TypedValue> {
    n.field(key, p)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Aspect actual source {key} absent"))
}
fn text(n: &ResolvedNode<'_>, key: &str, p: Precision) -> Result<String> {
    match value(n, key, p)? {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(format!("Aspect {key} requires native String")),
    }
}
fn rows<'a>(view: &ResolvedNode<'a>, p: Precision) -> Result<Vec<ResolvedNode<'a>>> {
    let mut result = vec![];
    if !value(view, "aspects_as_variations", p)?
        .boolean()
        .map_err(|e| e.to_string())?
    {
        result.push(
            view.child("AspectNormal")
                .map_err(|e| e.to_string())?
                .ok_or("49DC40 missing AspectNormal requires actual diagnostic")?,
        );
    }
    result.extend(view.children("Aspect").map_err(|e| e.to_string())?);
    Ok(result)
}
fn input(
    n: &ResolvedNode<'_>,
    view: &ResolvedNode<'_>,
    folder: &str,
    p: Precision,
) -> Result<Input> {
    let prefix = text(view, "resource_prefix", p)?;
    let aspect_prefix = text(n, "aspect_prefix", p)?;
    let model = super::path::model_path(folder, &prefix, &aspect_prefix, &text(n, "model", p)?)?;
    let visual_radius = match value(n, "visual_radius_estimate", p)? {
        TypedValue::Scalar(v) if v.field_type == ScalarType::Float => v.bits,
        _ => return Err("Aspect visual radius64 requires native Float source".into()),
    };
    let rename = n.children("TextureRename").map_err(|e| e.to_string())?;
    let texture_path = |key: &str| -> Result<String> {
        let mut s = text(n, key, p)?;
        //46A180 returns null BEFORE traversing even a positive rename list.
        if s.is_empty() {
            return Ok(s);
        }
        //469D10 substring replacement has a separately required positive
        //source proof; do not silently use the unrenamed path.
        if !rename.is_empty() {
            return Err(
                "46A180 positive TextureRename requires actual469D10 source producer".into(),
            );
        }
        let joined = if aspect_prefix.starts_with('/') {
            aspect_prefix.clone()
        } else {
            format!("{prefix}{aspect_prefix}")
        };
        s = super::path::expand(&s, &joined)?;
        super::path::relative(folder, &s)
    };
    let mut texture_paths = vec![];
    for path in view
        .children("TextureSearchPath")
        .map_err(|e| e.to_string())?
    {
        let path = text(&path, "path", p)?;
        if path.is_empty() {
            continue;
        }
        //49DC40 tokenizes this source. A single path is the bounded admitted
        //domain until its native delimiter/parser proof is composed.
        if path.chars().any(char::is_whitespace) {
            return Err(
                "Object TextureSearchPath positive tokenization requires actual424FA0 fields"
                    .into(),
            );
        }
        texture_paths.push(super::path::relative(folder, &path)?);
    }
    Ok(Input {
        sources: n.sources(),
        model: model.encode_utf16().collect(),
        visual_radius,
        skin_paths: [texture_path("skin")?, texture_path("glow")?],
        texture_paths,
        resource_prefix: prefix,
        folder: folder.into(),
    })
}
impl Row {
    fn constructed() -> Self {
        //Relevant ordinary46A510 fields. Model/skin/animation pointers are
        //actual constructor nulls; count18/extra4C/extra54 are native zero.
        Self {
            input: None,
            count: 0,
            biome: None,
            model: None,
            skins: [None, None],
            skin_stage: 0,
            animations: None,
            animation_stage: 0,
            source_ready: false,
            extra_count: 0,
            extra_requested: false,
        }
    }
}
impl State {
    ///Source-phase selected Aspect field producer. Run before any World biome
    ///context change: skin/glow and Sequence fields are source Loader pointers.
    ///An error retains the unpublished pending Aspect and its original child
    ///allocations. This is not full Object/ActorData readiness or vf14 success.
    pub fn prepare_object_aspects(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        resources: &mut Resources<'_>,
    ) -> Result<()> {
        let data = self.bind_object(catalog, bank, d)?;
        let pc = bank.combat_source()?.source_precision();
        let root = node(catalog, d)?;
        let view = root.child("View").map_err(|e| e.to_string())?;
        if self.entry(&data)?.aspects.is_none() {
            self.entry_mut(&data)?.aspects = Some(Object {
                definition: d.clone(),
                rows: vec![],
                pending: None,
                complete: false,
            });
        }
        let owner = self.entry_mut(&data)?.aspects.as_mut().unwrap();
        if owner.complete {
            return Ok(());
        }
        let Some(view) = view else {
            owner.complete = true;
            return Ok(());
        };
        let nodes = rows(&view, pc)?;
        let normalized = d.proof.source.path.replace('\\', "/");
        let folder = normalized.rsplit_once('/').map_or("", |(f, _)| f);
        while owner.rows.len() < nodes.len() {
            let n = &nodes[owner.rows.len()];
            if owner.pending.is_none() {
                owner.pending = Some(Row::constructed());
            }
            let row = owner.pending.as_mut().unwrap();
            if row.input.is_none() {
                row.input = Some(input(n, &view, folder, pc)?);
            }
            let input = row.input.as_ref().unwrap();
            if input.sources != n.sources() {
                return Err("pending Aspect immutable source changed".into());
            }
            while row.skin_stage < 2 {
                let ids = &input.skin_paths[row.skin_stage];
                let handle = if ids.is_empty() {
                    None
                } else {
                    let found = resources.texture_5a77e0(
                        catalog,
                        bank,
                        &ids.encode_utf16().collect::<Vec<_>>(),
                        1,
                    )?;
                    if found.is_none() && !input.texture_paths.is_empty() {
                        return Err(
                            "5A79F0 actual skin search-path miss coordinator required".into()
                        );
                    }
                    if found.is_none() {
                        return Err("5A79F0 final original Texture request remains required".into());
                    }
                    found
                };
                row.skins[row.skin_stage] = handle;
                row.skin_stage += 1;
            }
            let animations = n.children("Animation").map_err(|e| e.to_string())?;
            while row.animation_stage < animations.len() {
                if row.animations.is_none() {
                    row.animations = Some(
                        crate::object_animation_set::Set::from_native_constructor(&data),
                    );
                }
                row.animations.as_mut().unwrap().load_source_node(
                    catalog,
                    bank,
                    &animations[row.animation_stage],
                    &input.resource_prefix,
                    &input.folder,
                    resources,
                )?;
                row.animation_stage += 1;
            }
            //The remaining ground-size/disable/type/weight fields are outside
            //this selected field owner, not fabricated constructor values.
            row.source_ready = true;
            owner.rows.push(owner.pending.take().unwrap());
        }
        let banner = text(&view, "banner_ids", pc)?;
        if !banner.is_empty() {
            return Err(
                "49DC40 positive Banner requires actual46A650 replacement/destruction owner".into(),
            );
        }
        owner.complete = true;
        Ok(())
    }
    pub fn object_aspect(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        index: usize,
    ) -> Result<Aspect> {
        let data = self.bind_object(catalog, bank, d)?;
        let object = self
            .entry(&data)?
            .aspects
            .as_ref()
            .ok_or("actual source Aspect allocation table not prepared")?;
        if !object.complete {
            return Err("actual source Aspect list has unfinished child/replacement".into());
        }
        object
            .rows
            .get(index)
            .ok_or("actual Aspect index outside source allocation table")?;
        Ok(Aspect { data, index })
    }
    pub fn aspect_visual_radius_64(&self, aspect: &Aspect) -> Result<u32> {
        let row = self.aspect_row(aspect)?;
        Ok(row
            .input
            .as_ref()
            .ok_or("Aspect source radius64 not prepared")?
            .visual_radius)
    }
    fn aspect_row(&self, aspect: &Aspect) -> Result<&Row> {
        let object = self
            .entry(&aspect.data)?
            .aspects
            .as_ref()
            .ok_or("Aspect owner absent")?;
        if !object.complete {
            return Err("Aspect source list unfinished".into());
        }
        object
            .rows
            .get(aspect.index)
            .ok_or_else(|| "Aspect allocation index absent".into())
    }
    ///469FE0 over the actual stored source Aspect/Model/Biome identities.
    pub fn resolve_aspect_model(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        aspect: &Aspect,
        biome: Option<&Biome>,
        resources: &mut Resources<'_>,
        policy: &mut impl ModelPolicy,
    ) -> Result<Handle> {
        let data = aspect.data.clone();
        self.entry(&data)?;
        let object = self
            .entries
            .get_mut(&data.token)
            .unwrap()
            .aspects
            .as_mut()
            .ok_or("Aspect allocation owner absent")?;
        crate::live_object_init::Source::bind_in_bank(catalog, bank, &object.definition)
            .map_err(|e| e.to_string())?;
        if !object.complete {
            return Err("Aspect source fields have not completed".into());
        }
        let row = object
            .rows
            .get_mut(aspect.index)
            .ok_or("Aspect index absent")?;
        let mut backend = RequiredBackend;
        let mut world = Bound {
            row,
            catalog,
            bank,
            biome,
            resources,
            policy,
            backend: &mut backend,
            suppressed: &mut self.aspect_assertion_suppressed,
        };
        fields::resolve(&mut world)?;
        world
            .row
            .model
            .clone()
            .ok_or_else(|| "469FE0 current Model28 absent".into())
    }
    ///454F70 performs another real5A8520 on current Model+38 after469FE0.
    ///The returned handle belongs to that factory call, not a copied pointer.
    pub fn model_for_view454f70(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        aspect: &Aspect,
        biome: Option<&Biome>,
        resources: &mut Resources<'_>,
        policy: &mut impl ModelPolicy,
    ) -> Result<Handle> {
        let model = self.resolve_aspect_model(catalog, bank, aspect, biome, resources, policy)?;
        let ids = resources.model_filename_38(&model)?;
        resources.model_5a8520(catalog, bank, &ids, policy)
    }
    ///46AEC0 through actual Resource/Sequence vf10/vf0C services. No native
    ///loaded/bounds bits are inferred from source presence or modern uploads.
    pub fn request_aspect(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        aspect: &Aspect,
        biome: Option<&Biome>,
        resources: &mut Resources<'_>,
        policy: &mut impl ModelPolicy,
        backend: &mut impl Backend,
        delta: u32,
        mode: u32,
    ) -> Result<()> {
        self.entry(&aspect.data)?;
        let object = self
            .entries
            .get_mut(&aspect.data.token)
            .unwrap()
            .aspects
            .as_mut()
            .ok_or("Aspect allocation owner absent")?;
        crate::live_object_init::Source::bind_in_bank(catalog, bank, &object.definition)
            .map_err(|e| e.to_string())?;
        if !object.complete {
            return Err("Aspect source fields have not completed".into());
        }
        let row = object
            .rows
            .get_mut(aspect.index)
            .ok_or("Aspect index absent")?;
        fields::request(
            &mut Bound {
                row,
                catalog,
                bank,
                biome,
                resources,
                policy,
                backend,
                suppressed: &mut self.aspect_assertion_suppressed,
            },
            delta,
            mode,
        )
    }
}
struct RequiredBackend;
impl crate::resource_task::Services for RequiredBackend {}
impl Backend for RequiredBackend {}
struct Bound<'a, 'source, 'manager, P, B> {
    row: &'a mut Row,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    biome: Option<&'a Biome>,
    resources: &'a mut Resources<'manager>,
    policy: &'a mut P,
    backend: &'a mut B,
    suppressed: &'a mut bool,
}
impl<P: ModelPolicy, B: Backend> fields::World for Bound<'_, '_, '_, P, B> {
    type Model = Handle;
    type Biome = BiomeIdentity;
    fn context_sensitive(&mut self) -> Result<bool> {
        Ok(true)
    }
    fn current_biome(&mut self) -> Result<BiomeIdentity> {
        self.biome
            .ok_or("469FE0 requires actual retained current GMap Biome")?
            .identity(self.catalog, self.bank)
    }
    fn stored_biome(&mut self) -> Result<Option<BiomeIdentity>> {
        Ok(self.row.biome.clone())
    }
    fn same_biome(&self, a: &BiomeIdentity, b: &BiomeIdentity) -> bool {
        a.same_identity(b)
    }
    fn store_biome(&mut self, b: BiomeIdentity) -> Result<()> {
        self.row.biome = Some(b);
        Ok(())
    }
    fn model(&mut self) -> Result<Option<Handle>> {
        Ok(self.row.model.clone())
    }
    fn same_model(&self, a: &Handle, b: &Handle) -> bool {
        a.same_identity(b)
    }
    fn make_model(&mut self) -> Result<Handle> {
        self.resources.model_5a8520(
            self.catalog,
            self.bank,
            &self
                .row
                .input
                .as_ref()
                .ok_or("Aspect source Model20 not prepared")?
                .model,
            self.policy,
        )
    }
    fn store_model(&mut self, v: Handle) -> Result<()> {
        self.row.model = Some(v);
        Ok(())
    }
    fn model_flags(&mut self, v: &Handle) -> Result<u32> {
        self.resources.flags_08(v)
    }
    fn mark_model(&mut self, v: &Handle) -> Result<()> {
        self.resources
            .aspect_model_flags(v, false, self.row.skins[0].is_some())
    }
    fn extra_requested(&mut self) -> Result<bool> {
        Ok(self.row.extra_requested)
    }
    fn assertion_suppressed(&mut self) -> Result<bool> {
        Ok(*self.suppressed)
    }
    fn assert_context_change(&mut self) -> Result<()> {
        Err(
            "469FE0 context change requires actual855D30 assertion feedback; latch unchanged"
                .into(),
        )
    }
    fn clear_extra_count(&mut self) -> Result<()> {
        self.row.extra_count = 0;
        Ok(())
    }
    fn store_extra_requested(&mut self, v: bool) -> Result<()> {
        self.row.extra_requested = v;
        Ok(())
    }
    fn request_skin(&mut self, index: usize, delta: u32, mode: u32) -> Result<()> {
        if let Some(handle) = &self.row.skins[index] {
            self.resources
                .request_10(handle, delta as i32, mode, self.backend)?;
        }
        Ok(())
    }
    fn request_animations(&mut self, delta: u32) -> Result<()> {
        if let Some(set) = &mut self.row.animations {
            set.request(delta, self.resources, self.backend)?;
        }
        Ok(())
    }
    fn request_model(&mut self, handle: &Handle, delta: u32, mode: u32) -> Result<()> {
        self.resources
            .request_10(handle, delta as i32, mode, self.backend)
    }
    fn request_extra(&mut self, _: u32, _: u32) -> Result<()> {
        Err("Aspect loaded Model extra-texture46AE20 requires actual embedded-texture owner".into())
    }
    fn count(&mut self) -> Result<u32> {
        Ok(self.row.count)
    }
    fn store_count(&mut self, v: u32) -> Result<()> {
        self.row.count = v;
        Ok(())
    }
}

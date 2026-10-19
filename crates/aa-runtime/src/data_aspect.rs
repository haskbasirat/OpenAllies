//!Selected Aspect46A510/46A8A0 fields and their SAME application's469FE0
//!model owner. This does not certify completion of ObjectData::LoadData.
//!Banner replacement retains the actual Banner source/skin/model, then frees
//!the old selected source allocations before publishing its sole new Aspect.
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
#[path = "data_aspect_size_fields.rs"]
mod size_fields;
#[path = "data_aspect_rename.rs"]
mod rename;
#[path = "data_aspect_release.rs"]
mod release;
use rename::{TextureRename, TextureRenames};
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
    context_sensitive: bool,
    unique: bool,
    ground_size: [u32; 2],
    ground_radius: u32,
}
struct Input {
    ids: String,
    sources: Vec<Provenance>,
    model: Vec<u16>,
    visual_radius: u32,
    ///46A180 results for +30 skin / +34 glow: the exact5A79F0 name, or
    ///None for the native early null (empty source, no request).
    skin_paths: [Option<String>; 2],
    texture_paths: Vec<String>,
    resource_prefix: String,
    ///Aspect+4. 46A180 flag1 formats with Aspect+0 (this source's
    ///resource_prefix copy) + Aspect+4, without the model's '/' rule.
    aspect_prefix: String,
    ///Aspect+3C TextureRename list, head first. Retained for later46A180
    ///flag0 callers (46AE20 loaded-Model texture names).
    renames: TextureRenames,
    folder: String,
}
impl Input {
    ///46A180 on this Aspect: flag1 (skin/glow) or flag0 (46AE20). `folder`
    ///is the 0xB36530 source context current at the call.
    pub(crate) fn texture_name_46a180(
        &self,
        source: &str,
        flag: bool,
        folder: &str,
    ) -> Result<Option<String>> {
        let prefix = format!("{}{}", self.resource_prefix, self.aspect_prefix);
        self.renames
            .texture_name_46a180(source, flag.then_some(prefix.as_str()), folder)
    }
}
///One stored46A180 name as its5A79F0 child receives it. The empty name a
///positive source can produce (rename/format to nothing) is still a native
///request; its5A77E0 behaviour is unproven, so it is refused explicitly.
fn texture_request_name(name: &Option<String>) -> Result<Option<Vec<u16>>> {
    match name {
        None => Ok(None),
        Some(name) if name.is_empty() => Err(
            "5A79F0 empty Texture name from a positive46A180 source requires actual5A77E0 proof"
                .into(),
        ),
        Some(name) => Ok(Some(name.encode_utf16().collect())),
    }
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
                return Err("Aspect canonical source position is ambiguous".into());
            }
            selected = Some(n);
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
fn flag_variations(view: &ResolvedNode<'_>, p: Precision) -> Result<bool> {
    value(view, "aspects_as_variations", p)?
        .boolean()
        .map_err(|e| e.to_string())
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
    //46A8A0 reads every TextureRename child (substring_from into node+0,
    //then substring_to into node+4) after weight and before skin/glow.
    let mut renames = vec![];
    for child in n.children("TextureRename").map_err(|e| e.to_string())? {
        renames.push(TextureRename {
            from: text(&child, "substring_from", p)?.encode_utf16().collect(),
            to: text(&child, "substring_to", p)?.encode_utf16().collect(),
        });
    }
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
    let mut input = Input {
        ids: text(n, "IDS", p)?,
        sources: n.sources(),
        model: model.encode_utf16().collect(),
        visual_radius,
        skin_paths: [None, None],
        texture_paths,
        resource_prefix: prefix,
        aspect_prefix,
        renames: TextureRenames::from_source_order(renames),
        folder: folder.into(),
    };
    //46A180(skin,1) ->+30, then46A180(glow,1) ->+34, with this source's
    //folder as the current 0xB36530 context.
    let skin = input.texture_name_46a180(&text(n, "skin", p)?, true, folder)?;
    let glow = input.texture_name_46a180(&text(n, "glow", p)?, true, folder)?;
    input.skin_paths = [skin, glow];
    Ok(input)
}
impl Row {
    fn constructed_banner() -> Self {
        let mut row = Self::constructed();
        row.context_sensitive = false;
        row.unique = true;
        row
    }
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
            context_sensitive: true,
            unique: false,
            ground_size: [0; 2],
            ground_radius: 0,
        }
    }
}
fn store_size(row: &mut Row, size: [u32; 2], pc: Precision) -> Result<()> {
    row.ground_size = size;
    row.ground_radius = size_fields::radius(size, pc)?;
    Ok(())
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
        self.prepare_object_aspects_with_policy(
            catalog,
            bank,
            d,
            resources,
            &mut RequiredTexturePolicy,
        )
    }
    pub fn prepare_object_aspects_with_policy(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        resources: &mut Resources<'_>,
        texture_policy: &mut impl crate::resource_objects::TexturePolicy,
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
                let handle = if let Some(ids) =
                    texture_request_name(&input.skin_paths[row.skin_stage])?
                {
                    let found = resources.texture_5a77e0_with_policy(
                        catalog,
                        bank,
                        &ids,
                        1,
                        texture_policy,
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
                } else {
                    None
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
                //5A80D0 caught missing-Sequence path (DataCompatibilityMode)
                //returns NULL and49DA40 skips that ObjectAnimation.
                row.animations.as_mut().unwrap().load_source_node_with_policy(
                    catalog,
                    bank,
                    &animations[row.animation_stage],
                    &input.resource_prefix,
                    &input.folder,
                    resources,
                    texture_policy,
                )?;
                row.animation_stage += 1;
            }
            //46A8A0 stores these after Animation children, before the Banner
            //caller conditionally copies the selected normal Aspect's size.
            let size = match value(n, "ground_size", pc)? {
                TypedValue::Point { bits, .. }
                    if bits.len() == 2 && bits.iter().all(|v| f32::from_bits(*v).is_finite()) =>
                {
                    [bits[0], bits[1]]
                }
                _ => return Err("Aspect ground_size requires actual finite Point2".into()),
            };
            store_size(row, size, pc)?;
            //Disable/type/weight fields remain outside this selected owner.
            row.source_ready = true;
            owner.rows.push(owner.pending.take().unwrap());
        }
        let banner = text(&view, "banner_ids", pc)?;
        if banner.is_empty() {
            //45D3F0/45DAF0 actual Data vf20. Character's bit2 override
            //precedes the category descriptor's view type at+20.
            let view_type = if d.component_mask.is_some_and(|m| m & 4 != 0) {
                4
            } else {
                d.proof.category.view_type
            };
            if view_type == 21 {
                return Err("49E6D1 Organization View requires an actual Banner".into());
            }
            if owner.rows.is_empty() {
                return Err("49E6EF Object requires at least one actual Aspect".into());
            }
            owner.complete = true;
            return Ok(());
        }
        //Release the short loan before binding the SAME DataState's separate
        //canonical Banner directory. Native+134 stores before later guards.
        let _ = owner;
        let reference = self.bind_banner(catalog, bank, &banner)?;
        self.entry_mut(&data)?.banner = Some(reference.clone());
        if flag_variations(&view, pc)? {
            return Err("49E5DF Banner object must not use aspects as variations".into());
        }
        self.prepare_banner(catalog, bank, &reference, resources, texture_policy)?;
        let (model, skin, sources, banner_folder) =
            self.banner_aspect_input(catalog, bank, &reference)?;
        let owner = self.entry_mut(&data)?.aspects.as_mut().unwrap();
        if owner.pending.is_none() {
            let mut row = Row::constructed_banner();
            row.input = Some(Input {
                ids: banner,
                sources,
                model,
                visual_radius: 0,
                skin_paths: [None, None],
                texture_paths: vec![],
                resource_prefix: String::new(),
                //46A650 BannerAspect never builds a TextureRename list.
                aspect_prefix: String::new(),
                renames: TextureRenames::default(),
                folder: banner_folder,
            });
            row.skins = [skin, None];
            row.skin_stage = 2;
            row.source_ready = true;
            owner.pending = Some(row);
        }
        //Native normal index is captured immediately after AspectNormal
        //append; non-variation source order places that actual row at zero.
        if let Some(normal) = owner.rows.first() {
            let size = normal.ground_size;
            if f32::from_bits(size[0]) > 0.0 || f32::from_bits(size[1]) > 0.0 {
                store_size(owner.pending.as_mut().unwrap(), size, pc)?;
            }
        }
        //46A770/49C270 free only these source allocations/strings/vectors.
        //They do not call Resource release; Manager counters remain owned.
        //Any unexpected already-requested Aspect needs its real destructor
        //continuation before an old allocation may be discarded here.
        for row in &owner.rows {
            if !row.source_ready
                || row.count != 0
                || row.model.is_some()
                || row.extra_requested
                || row.extra_count != 0
            {
                return Err("46A770 old Aspect has live request/model fields; actual destruction owner required".into());
            }
        }
        owner.rows.clear();
        owner.rows.push(owner.pending.take().unwrap());
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
    ///Metadata from the SAME prepared Aspect; no authored row is reselected.
    pub fn aspect_ids(&self, aspect: &Aspect) -> Result<&str> {
        Ok(&self
            .aspect_row(aspect)?
            .input
            .as_ref()
            .ok_or("Aspect source fields absent")?
            .ids)
    }
    pub fn aspect_source_folder(&self, aspect: &Aspect) -> Result<&str> {
        Ok(&self
            .aspect_row(aspect)?
            .input
            .as_ref()
            .ok_or("Aspect source fields absent")?
            .folder)
    }
    pub fn aspect_visual_radius_64(&self, aspect: &Aspect) -> Result<u32> {
        let row = self.aspect_row(aspect)?;
        Ok(row
            .input
            .as_ref()
            .ok_or("Aspect source radius64 not prepared")?
            .visual_radius)
    }
    ///Immutable names only, for ActorData49BF90 slot resolution. This neither
    ///duplicates the mutable Aspect table nor owns a second Model pointer.
    pub(crate) fn prepared_aspect_layout(&self, data: &Data) -> Result<Layout> {
        let object = self
            .entry(data)?
            .aspects
            .as_ref()
            .ok_or("actual Aspect table not prepared")?;
        if !object.complete {
            return Err("actual Aspect table source is unfinished".into());
        }
        let names = object
            .rows
            .iter()
            .map(|r| {
                r.input
                    .as_ref()
                    .map(|i| i.ids.clone())
                    .ok_or_else(|| "prepared Aspect has no source IDS".to_owned())
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Layout {
            data: data.clone(),
            definition: object.definition.clone(),
            names,
        })
    }
    pub(super) fn aspect_count_word(&self, aspect: &Aspect) -> Result<u32> {
        Ok(self.aspect_row(aspect)?.count)
    }
    pub(super) fn aspect_request_count(&self, data: &Data) -> Result<isize> {
        let object = self
            .entry(data)?
            .aspects
            .as_ref()
            .ok_or("actual source Aspect table absent")?;
        if !object.complete {
            return Err("actual source Aspect table unfinished".into());
        }
        isize::try_from(object.rows.len())
            .map_err(|_| "Aspect count outside native signed domain".into())
    }
    pub(super) fn aspect_current_model(&self, data: &Data, index: usize) -> Result<Handle> {
        let row = self.aspect_row(&Aspect {
            data: data.clone(),
            index,
        })?;
        row.model
            .as_ref()
            .cloned()
            .ok_or_else(|| "actual current Aspect Model28 is null".into())
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
        super::validate_object_data(catalog, bank, &object.definition)?;
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
        super::validate_object_data(catalog, bank, &object.definition)?;
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
pub(crate) struct Layout {
    data: Data,
    definition: RuntimeDefinition,
    names: Vec<String>,
}
impl Layout {
    pub(crate) fn definition(&self) -> &RuntimeDefinition {
        &self.definition
    }
    pub(crate) fn require_data(&self, data: &Data) -> Result<()> {
        if !self.data.same_allocation(data) {
            return Err("Actor animation layout belongs to another Data allocation".into());
        }
        Ok(())
    }
    pub(crate) fn names(&self) -> &[String] {
        &self.names
    }
}
struct RequiredTexturePolicy;
impl crate::resource_objects::TexturePolicy for RequiredTexturePolicy {}
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
        Ok(self.row.context_sensitive)
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
            .aspect_model_flags(v, self.row.unique, self.row.skins[0].is_some())
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

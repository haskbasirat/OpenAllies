//! One application's accepted source backend and generic resource namespace.
//! Map/film names are checked against the SAME namespace, never independent
//! type-separated lookups. This does not certify native worker/startup phases.
use crate::{
    catalog::Catalog,
    film_create::{Depot, DepotId},
    film_resource::Resource,
    load_view_variables::NativeOverrides,
    map_resource_streams,
    resource_creator::SourceResource,
    resource_map_cache,
};
use aa_rules::sources::V14Sources;
use std::{
    collections::BTreeMap,
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, Weak},
};
type Result<T> = std::result::Result<T, String>;

///Move-only source/depot owner, retained across selected-map probes and films.
///No empty sources, cloned archive backend, or caller-inserted cache entry.
pub struct Application {
    origin: Arc<()>,
    sources: V14Sources,
    maps: resource_map_cache::State,
    manager: Mutex<Manager>,
    process: Option<crate::film_process_metadata::State>,
    ///Sole canonical Data+20 scalar directory, independent of resource leases
    ///and of the resource cache lock needed by its reached child requests.
    data_requests:Arc<Mutex<crate::data_pre_request::State>>,
    resource_priority:crate::resource_task::DefaultPriority,
    resource_tasks:crate::resource_task::TaskQueue,
    resource_guards:crate::resource_task_guards::State,
    cache_variables:Option<crate::resource_audio_class::FreshCacheVariableScope>,
    model_source_context:Option<crate::resource_model_source::Context>,
    ///The one portable initialized8C3ED0 property prefix. Models retain only
    ///loans of these same allocations, never per-model replacement defaults.
    model_defaults:crate::resource_model_defaults::State,
}
struct Directory {
    id: DepotId,
    path: PathBuf,
    writable: i32,
    priority: usize,
}
///Sealed read capability. Its Arc aliases the one DataFXSession mutex; the
///Weak Application key rejects use after the actual manager is destroyed.
pub struct SourceGraphsRead{application:Weak<()>,state:Arc<Mutex<crate::data_pre_request::State>>}
impl SourceGraphsRead{
    pub(crate) fn with_state<R>(&self,call:impl FnOnce(&crate::data_pre_request::State)->Result<R>)->Result<R>{
        self.application.upgrade().ok_or("source Graph Application owner is retired")?;
        let state=self.state.try_lock().map_err(|e|format!("actual source Graph/Data owner is already borrowed/poisoned: {e}"))?;
        call(&state)
    }
}
#[cfg(test)]
#[path="resource_audio_manager_tests.rs"]
mod audio_manager_tests;
struct Manager {
    films: BTreeMap<String, Resource>,
    resources:BTreeMap<String,crate::resource_objects::Entry>,
    work: Option<Directory>,
    film_class: Option<crate::resource_cache_metadata::FilmClass>,
    texture_class:Option<crate::resource_texture_class::TextureClass>,
    audio_class:Option<crate::resource_audio_class::AudioClass>,
    model_class:Option<crate::resource_model_class::ModelClass>,
    ni_cache:crate::resource_ni_cache::State,
    texture_objects:crate::resource_texture_owner::State,
    ///KKC_ResourceCacheManager at native ResourceManager+9C.
    cache:cache::State,
    warnings: Vec<String>,
    ///Native404FC0 calls reached by a caught missing-source path, in order.
    missing_resource_diagnostics: Vec<MissingResourceDiagnostic>,
    texture_subfolder_80: crate::vworld_resource_context::State,
    ///Modern backend residency is not silently classified as native Ni. Its
    ///registration remains until a real unload owner is implemented, even if
    ///a requesting view disappears. Weak expiry is not GPU deallocation.
    view_resources:BTreeMap<String,ViewResourceRegistration>,
}
struct ViewResourceRegistration {definition:aa_rules::resources::Provenance,leases:Vec<Weak<()>>}
///The log channel object a native404FC0 call receives.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiagnosticChannel {
    ///B38C28, also used by5A6E90's missing-Texture diagnostic.
    ResourceB38C28,
    ///B38B98, used by the AudioResource.cpp diagnostics.
    AudioB38B98,
}
///One native404FC0(channel,level,file,line,function,format,name) call made
///by a caught missing-source path, with every argument kept. 404FC0 itself
///only forwards to the channel's vf4 writer when level<=channel+C. It changes
///no game state, so these records are only diagnostics, never a resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissingResourceDiagnostic {
    pub channel: DiagnosticChannel,
    pub level: u32,
    pub file: &'static str,
    pub line: u32,
    pub function: &'static str,
    pub format: &'static str,
    pub ids: Vec<u16>,
}
impl MissingResourceDiagnostic {
    ///The text after the single native `%s` is filled with the requested name.
    pub fn message(&self) -> String {
        self.format.replacen("%s", &String::from_utf16_lossy(&self.ids), 1)
    }
}
///405D20/5A6E90 level: DataMissingResourceWarning true gives1, false10
///(`neg al; sbb eax,eax; and eax,-9; add eax,10`).
fn missing_resource_level(warning: u8) -> u32 {
    if warning != 0 { 1 } else { 10 }
}
fn require_no_unbound_view_invalidation(resources:&BTreeMap<String,ViewResourceRegistration>)->Result<()> {
    if !resources.is_empty(){return Err("5ACA50 modern view/model residency requires actual native class/refcounts and same-owner GPU unload; new+80 retained".into());}
    Ok(())
}
///5ACA50 over the SAME Manager+60 namespace, reached only after5AE0A0 has
///copied the changed+80 subfolder. Map/Film private owners have no generic
///insertion API; Map B3D7DC->B38E24 and Film B3D7F0->B3D7DC->B38E24 never
///reach B3D990 and skip before +4/+8. Concrete entries take the exact native
///per-entry predicate. Bucket order is unobservable while every entry skips.
fn invalidate_namespace_5aca50(maps:&resource_map_cache::State,films:&BTreeMap<String,Resource>,
    resources:&BTreeMap<String,crate::resource_objects::Entry>,view_resources:&BTreeMap<String,ViewResourceRegistration>)->Result<(usize,usize)>{
    let maps=maps.keys().count();
    let films=films.values().count();
    require_no_unbound_view_invalidation(view_resources)?;
    crate::resource_model_unload::invalidate_5aca50(resources.values())?;
    Ok((maps,films))
}
///Move-only same-application registration retained by the actual View owner.
///This does not certify native Ni class/refcounts, Init, upload, or readiness.
pub struct ViewResourceLease {application:Weak<()>,identity:Arc<()>,key:String,
    definition:aa_rules::resources::Provenance,world:crate::world_revision::Witness}
#[derive(Clone)]
pub struct ViewResourceWitness {application:Weak<()>,identity:Weak<()>,key:String,
    definition:aa_rules::resources::Provenance}
impl ViewResourceLease {
    pub fn witness(&self)->ViewResourceWitness {ViewResourceWitness {application:self.application.clone(),identity:Arc::downgrade(&self.identity),key:self.key.clone(),definition:self.definition.clone()}}
    pub fn is_for(&self,world:&crate::world_revision::ConstructedWorldRevision)->bool {
        self.application.upgrade().is_some()&&self.world.is_for(world)
    }
}
impl ViewResourceWitness {
    pub fn validate(&self,ids:&str,definition:&aa_rules::resources::Provenance)->Result<()> {
        if self.application.upgrade().is_none()||self.identity.upgrade().is_none()
            ||self.key!=resource_map_cache::normalized_key(&ids.encode_utf16().collect::<Vec<_>>())?
            ||&self.definition!=definition {return Err("model upload registration/view lifetime/source differs".into());}
        Ok(())
    }
}
impl Application {
    ///Adopt the actual already-verified source backend once. Native5B0400's
    ///name hashes start empty; this projection is not full ResourceManager
    ///construction, work-depot setup or an invented successful IO worker.
    pub fn from_sources(sources: V14Sources) -> Self {
        let origin=Arc::new(());
        let resource_priority=crate::resource_task::DefaultPriority::from_fresh_application(&origin);
        let resource_tasks=crate::resource_task::TaskQueue::from_fresh_application(&resource_priority).expect("same live Application priority owner");
        let resource_guards=crate::resource_task_guards::State::from_fresh_application(&origin);
        Self {
            origin:Arc::clone(&origin),
            resource_priority,
            resource_tasks,
            resource_guards,
            cache_variables:None,
            model_source_context:None,
            model_defaults:crate::resource_model_defaults::State::initialized_prefix(&origin),
            sources,
            maps: resource_map_cache::State::constructed(),
            manager: Mutex::new(Manager {
                films: BTreeMap::new(),
                resources:BTreeMap::new(),
                work: None,
                film_class: None,
                texture_class:None,
                audio_class:None,
                model_class:None,
                ni_cache:crate::resource_ni_cache::State::from_fresh_application(&origin),
                cache:cache::State::default(),
                texture_objects:crate::resource_texture_owner::State::from_fresh_application(&origin),
                warnings: Vec::new(),
                missing_resource_diagnostics: Vec::new(),
                texture_subfolder_80: crate::vworld_resource_context::State::constructed(),
                view_resources:BTreeMap::new(),
            }),
            process: None,
            data_requests:Arc::new(Mutex::new(crate::data_pre_request::State::from_fresh_application(&origin))),
        }
    }
    pub fn sources_mut(&mut self) -> &mut V14Sources {
        &mut self.sources
    }
    ///Copies of the native404FC0 calls made so far by caught missing-source
    ///paths (5A80D0 and59B4B0/59B280), in call order.
    pub fn missing_resource_diagnostics(&self) -> Result<Vec<MissingResourceDiagnostic>> {
        Ok(self.manager.lock().map_err(|_| "Resource manager lock poisoned")?.missing_resource_diagnostics.clone())
    }
    ///Actual completed pre-World source interval. The context is installed
    ///once before any Model allocation, and never inferred from source roots.
    pub fn prepare_model_source_context(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank)->Result<()>{
        if self.model_source_context.is_some(){return Err("same Application Model source context already bound".into())}
        if self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.resources.values().any(|entry|entry.model.is_some()){
            return Err("Model Bone source context must precede Model resource publication".into())
        }
        let loading=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.ni_cache.loading_context_capability();
        let context=crate::resource_model_source::Context::bind(catalog,bank,&self.origin,self.model_defaults.capability(),loading)?;
        self.model_source_context=Some(context);Ok(())
    }
    ///Install only the actual fresh no-profile source-variable window. Class
    ///creation and the first weight read remain lazy after Resource publication.
    pub(crate) fn install_fresh_cache_variable_scope(&mut self,catalog:&Catalog<'_>,overrides:NativeOverrides)->Result<()>{
        if self.cache_variables.is_some(){return Err("ResourceCache fresh variable scope is already installed".into())}
        let manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        if manager.audio_class.is_some()||manager.resources.values().any(|e|e.sound.is_some()){
            return Err("ResourceCache fresh variable window must precede actual Sound publication".into())
        }
        drop(manager);
        let scope=crate::resource_audio_class::FreshCacheVariableScope::from_sources(catalog,&mut self.sources,overrides)?;
        self.cache_variables=Some(scope);Ok(())
    }
    ///Loan the SAME manager's actual recursive guard owner before borrowing
    ///Resources. This capability owns no Resource field/ready snapshot.
    pub fn resource_task_guards(&self)->crate::resource_task_guards::Capability {
        self.resource_guards.capability()
    }
    ///Explicit modern backend counterpart of native710433, after a genuine
    ///current adapter/device/pipeline capability. No bool or source default
    ///can trigger this transition; incomplete/stale capabilities leave1.
    pub fn apply_renderer_init_710433(&mut self,backend:&impl crate::resource_texture::RendererInitialization)->Result<()>{
        backend.require_current_backend()?;
        self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.texture_objects.renderer_init_710433();Ok(())
    }
    ///The actual specialized LoadGameTask's5AE160 publication advances the
    ///SAME application counter used by ResourceTask. Validate both the retained
    ///selected Creator and this manager's actual source allocation before that
    ///store. The caller retains the returned+C slot on that SAME task before
    ///publishing its real queue node; this is no completion/ready capability.
    pub fn publish_load_task_sequence(
        &self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        selected: &crate::resource_creator::Creator,
        inputs: &crate::load_game_task::Inputs,
    ) -> Result<crate::resource_task::ExternalSequence> {
        inputs.validate_creator(catalog, bank, selected)?;
        self.maps.require_source(inputs.source())?;
        self.resource_tasks.reserve_external_enqueue_5ae160()
    }
    pub(crate) fn enqueue_view_task_5aed10(&self,task:&crate::view_resource_task::Task)->Result<()>{
        self.resource_tasks.enqueue_view_5aed10(task)
    }
    ///The next actual heterogeneous queue tail; all Resource callbacks borrow
    ///this SAME manager/source facade. A refused child retains its queue prefix.
    pub fn dispatch_queued_task_5addd0<B:Backend+crate::resource_task::DispatchServices>(&mut self,backend:&mut B)
        ->Result<Option<crate::resource_task::DispatchReport>>{
        let tasks=&self.resource_tasks;
        let mut resources=Resources{origin:&self.origin,sources:&mut self.sources,maps:&mut self.maps,manager:&self.manager,
            priority:&self.resource_priority,tasks,cache_variables:&self.cache_variables,model_source_context:&self.model_source_context};
        tasks.dispatch_next_5addd0(&mut ResourceCallbacks{resources:&mut resources,backend})
    }
    ///Borrow the same application-lifetime Data counters while real child
    ///services independently borrow the existing resource manager. A reentrant
    ///second counter loan fails explicitly instead of blocking or replacing it.
    pub fn with_data_pre_request<R>(&self,call:impl FnOnce(&mut crate::data_pre_request::State)->Result<R>)->Result<R>{
        let mut state=self.data_requests.try_lock().map_err(|e|format!("actual Data pre-request owner is already borrowed/poisoned: {e}"))?;
        call(&mut state)
    }
    ///Read-only capability to the SAME Data allocation owner, independent of
    ///mutable source/depot sibling loans. Retaining it cannot keep App alive.
    pub fn source_graphs(&self)->SourceGraphsRead{SourceGraphsRead{application:Arc::downgrade(&self.origin),state:Arc::clone(&self.data_requests)}}
    ///43F0F1->5AE0A0 over this SAME manager. Map/Film allocations (including
    ///map Pending/Failed prefixes) and Texture/Sequence/Sound entries have
    ///class chains without NiAVObjResource B3D990, so5ACA50 skips each before
    ///reading flags/counts. A Model is unloaded only when it is a cached
    ///(bit5) unrequested (signed count<=0) entry; see resource_model_unload.
    ///A modern GPU cache may not be silently attached to this closed namespace.
    pub fn set_vworld_resource_context(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        biome: &crate::vworld_resource_context::Biome,
    ) -> Result<crate::vworld_resource_context::Update> {
        let input = crate::vworld_resource_context::Input::bind(catalog, bank, biome)?;
        let mut manager = self.manager.lock().map_err(|_| "resource manager lock poisoned")?;
        let Manager { films, resources, texture_subfolder_80,view_resources, .. } = &mut *manager;
        texture_subfolder_80.assign_and_invalidate(&input.units, || {
            invalidate_namespace_5aca50(&self.maps, films, resources, view_resources)
        })
    }
    ///43F0F1->5AE0A0 with the actual resource backend: every 5ACA50-eligible
    ///cached unrequested NiAVObj Resource is unloaded through 5ABFF0 after
    ///the +80 assignment, in this namespace's traversal order.
    pub fn set_vworld_resource_context_with(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        biome: &crate::vworld_resource_context::Biome,
        backend: &mut impl Backend,
    ) -> Result<crate::vworld_resource_context::Update> {
        let input = crate::vworld_resource_context::Input::bind(catalog, bank, biome)?;
        let mut eligible = Vec::new();
        let update = {
            let mut manager = self.manager.lock().map_err(|_| "resource manager lock poisoned")?;
            let Manager { films, resources, texture_subfolder_80, view_resources, .. } = &mut *manager;
            let maps = &self.maps;
            texture_subfolder_80.assign_and_invalidate(&input.units, || {
                let counts = (maps.keys().count(), films.values().count());
                require_no_unbound_view_invalidation(view_resources)?;
                eligible = crate::resource_model_unload::eligible_handles_5aca50(resources.values());
                Ok(counts)
            })?
        };
        let mut resources = self.resources();
        for handle in &eligible {
            resources.unload_5abff0(handle, backend)?;
        }
        Ok(update)
    }
    ///A copied diagnostic value, never another mutable manager authority.
    pub fn vworld_texture_subfolder(&self) -> Result<Vec<u16>> {
        Ok(self.manager.lock().map_err(|_| "resource manager lock poisoned")?
            .texture_subfolder_80.current().to_vec())
    }
    ///Register the actual source-selected model name before any backend load
    ///or GPU residency. Provenance is the canonical TGI model input, not a
    ///claim of a raw NIF content hash or fabricated native reference count.
    pub fn register_view_resource(&self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,
        world:&crate::world_revision::ConstructedWorldRevision,ids:&str,
        definition:&aa_rules::resources::Provenance)->Result<ViewResourceLease> {
        catalog.require_source_owner()?;
        if bank.combat_source()?.source_identities()!=catalog.actors.sources.as_slice()
            ||!catalog.actors.sources.iter().any(|v|v.sha256==definition.sha256) {
            return Err("view resource canonical source/Bank owner differs".into());
        }
        let key=resource_map_cache::normalized_key(&ids.encode_utf16().collect::<Vec<_>>())?;
        if key.is_empty(){return Err("view resource canonical model name is empty".into());}
        let mut manager=self.manager.lock().map_err(|_|"resource manager lock poisoned")?;
        if self.maps.contains_key(&key)||manager.films.contains_key(&key) {
            return Err("view model name collides with an actual Map/Film resource".into());
        }
        if manager.view_resources.get(&key).is_some_and(|v|&v.definition!=definition) {
            return Err("current view model cache has a different canonical source".into());
        }
        let identity=Arc::new(());
        manager.view_resources.entry(key.clone()).or_insert_with(||ViewResourceRegistration {definition:definition.clone(),leases:Vec::new()})
            .leases.push(Arc::downgrade(&identity));
        Ok(ViewResourceLease {application:Arc::downgrade(&self.origin),identity,key,definition:definition.clone(),world:world.witness()})
    }
    ///Disjoint real Data+20 and ResourceManager loans. Reentrant Data loans
    ///refuse; resource children use the same existing namespace and source depot.
    pub fn with_data_pre_request_resources<R>(&mut self,call:impl FnOnce(&mut crate::data_pre_request::State,&mut Resources<'_>)->Result<R>)->Result<R>{
        let mut data=self.data_requests.try_lock().map_err(|e|format!("actual Data pre-request owner is already borrowed/poisoned: {e}"))?;
        let mut resources=Resources{origin:&self.origin,sources:&mut self.sources,maps:&mut self.maps,manager:&self.manager,priority:&self.resource_priority,tasks:&self.resource_tasks,cache_variables:&self.cache_variables,model_source_context:&self.model_source_context};
        call(&mut data,&mut resources)
    }
    pub fn resources(&mut self)->Resources<'_>{Resources{origin:&self.origin,sources:&mut self.sources,maps:&mut self.maps,manager:&self.manager,priority:&self.resource_priority,tasks:&self.resource_tasks,cache_variables:&self.cache_variables,model_source_context:&self.model_source_context}}
    ///Install the SAME fresh application metadata owner before any selected
    ///MapResource constructor. Diagnostic value-only source projections may
    ///exist without this owner; they certify no native constructor counter.
    pub fn attach_process_metadata(
        &mut self,
        process: crate::film_process_metadata::State,
    ) -> std::result::Result<(), (crate::film_process_metadata::State, String)> {
        let failure = if self.process.is_some() {
            Some("application process metadata was already installed")
        } else if self.maps.keys().next().is_some() {
            Some("source resource construction preceded actual process counter attachment")
        } else if self.manager.lock().map_err(|_|()).map_or(true, |manager| !manager.films.is_empty()) {
            Some("film resource construction preceded actual process counter attachment or manager lock failed")
        } else if process.constructed_headers() != 0 {
            Some("process metadata is past the fresh pre-resource constructor boundary")
        } else {
            None
        };
        if let Some(error) = failure {
            return Err((process, error.into()));
        }
        self.process = Some(process);
        Ok(())
    }
    pub fn process_metadata(&self) -> Result<&crate::film_process_metadata::State> {
        self.process
            .as_ref()
            .ok_or_else(|| "actual application process metadata is unbound".into())
    }
    fn require_map_class(&mut self, ids: &[u16]) -> Result<()> {
        let key = resource_map_cache::normalized_key(ids)?;
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| "resource manager lock poisoned")?;
        if let Some(resource)=manager.resources.get_mut(&key){resource.increment_reference14();return Err("generic lookup reached actual Texture/Model Resource; requested class differs".into());}
        if manager.view_resources.contains_key(&key) {return Err("Scenario lookup reached actual modern view/model resource; native class/refcount service required".into());}
        if let Some(film) = manager.films.get_mut(&key) {
            film.increment_references_14_wrapping()?;
            return Err("59F570 current generic resource is Film, not Scenario; actual wrong-class diagnostic required".into());
        }
        Ok(())
    }
    pub fn select_map(
        &mut self,
        catalog: &Catalog<'_>,
        ids: &[u16],
    ) -> Result<Arc<SourceResource>> {
        self.require_map_class(ids)?;
        if let Some(process) = self.process.as_mut() {
            self.maps
                .select_map_with_process(catalog, &mut self.sources, ids, process)
        } else {
            self.maps.select_map(catalog, &mut self.sources, ids)
        }
    }
    pub fn find_scenario(
        &mut self,
        catalog: &Catalog<'_>,
        ids: &[u16],
    ) -> Result<Arc<SourceResource>> {
        self.require_map_class(ids)?;
        if let Some(process) = self.process.as_mut() {
            self.maps
                .find_scenario_with_process(catalog, &mut self.sources, ids, process)
        } else {
            self.maps.find_scenario(catalog, &mut self.sources, ids)
        }
    }
    pub fn open_map_streams(
        &self,
        source: &Arc<SourceResource>,
    ) -> Result<map_resource_streams::Owner> {
        self.maps.open_map_streams(source)
    }
    pub fn map_references_14(&self, ids: &[u16]) -> Result<u32> {
        self.maps.references(ids)
    }
    ///Actual generic-name ownership, including a partially parsed map whose
    ///native base was already published. No missing metadata is read here.
    pub fn map_names(&self) -> impl Iterator<Item = &str> {
        self.maps.keys()
    }
    ///A copied native filename value from the SAME current work-directory
    ///owner. This is for the fresh resource's no-depot fallback or a proven
    ///work-is-topmost path; it certifies no foreign resource depot selection.
    pub fn work_path(&self, ids: &[u16]) -> Result<PathBuf> {
        let manager = self
            .manager
            .lock()
            .map_err(|_| "resource manager lock poisoned")?;
        resolve_work_path(
            manager.work.as_ref().ok_or("actual work depot absent")?,
            ids,
        )
    }
    ///59FC90/5AA860's work fallback for the fresh DataText sidecar. The
    ///caller has already appended'.tgi' at its actual native string writer.
    ///Both accepted source files and the current work directory are checked.
    ///A positive existing DataFile remains required rather than false absence.
    pub fn prepare_data_text_path(&mut self, sidecar_ids: &[u16]) -> Result<WorkPath> {
        let key = resource_map_cache::normalized_key(sidecar_ids)?;
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| "resource manager lock poisoned")?;
        if let Some(resource)=manager.resources.get_mut(&key){resource.increment_reference14();return Err("generic lookup reached actual Texture/Model Resource; requested class differs".into());}
        if manager.view_resources.contains_key(&key) {return Err("DataText lookup reached actual modern view/model resource; native class/refcount service required".into());}
        if self.maps.contains_key(&key) {
            self.maps.reject_wrong_type_lookup(sidecar_ids)?;
            unreachable!()
        }
        if let Some(film) = manager.films.get_mut(&key) {
            film.increment_references_14_wrapping()?;
            return Err("DataText lookup reached FilmResource class, not DataFileResource".into());
        }
        if self.sources.catalog().iter().any(|file| {
            resource_map_cache::normalized_key(
                &file.logical_path.encode_utf16().collect::<Vec<_>>(),
            )
            .as_ref()
                == Ok(&key)
        }) {
            return Err("DataText source exists and requires actual5AEA90 DataFileResource load/class owner".into());
        }
        let work = manager
            .work
            .as_ref()
            .ok_or("DataText actual work depot absent")?;
        let path = resolve_work_path(work, sidecar_ids)?;
        if path.try_exists().map_err(|e| e.to_string())? {
            return Err("DataText work file exists and requires actual5AEA90 DataFileResource load/class owner".into());
        }
        Ok(WorkPath {
            origin: Arc::downgrade(&self.origin),
            depot: work.id.clone(),
            root: work.path.clone(),
            path,
            ids: sidecar_ids.to_vec(),
        })
    }
    ///Safe filesystem transport for59FDA0's unconditional859B30 deletion;
    ///the native Boolean result is ignored. Unbound paths remain errors.
    pub fn delete_work_file(&self, ids: &[u16]) -> Result<()> {
        let path = self.work_path(ids)?;
        let _ = std::fs::remove_file(path);
        Ok(())
    }
    ///Portable rewrite backend choice, configured by the actual shipped
    ///autoexec declaration `adddepot Userdata/ 1`. This does not execute the
    ///entire autoexec, import a native profile or modify its Documents folder.
    ///The caller supplies the rewrite application's real root directory.
    pub fn configure_portable_work_depot(&mut self, application_root: &Path) -> Result<()> {
        const SOURCE: &[u8] = include_bytes!(
            "../../../research/development10/resource-work-depot/native-autoexec-source.txt"
        );
        if aa_rules::sources::sha256(SOURCE)
            != "ddfe2793f538fbc932c08ad559cf89e0a695719345197d47f35613dccfffe38e"
            || !SOURCE
                .split(|&b| b == b'\n')
                .any(|l| l.strip_suffix(b"\r").unwrap_or(l) == b"adddepot Userdata/ 1")
        {
            return Err("portable work-depot source declaration differs".into());
        }
        let mut manager = self
            .manager
            .lock()
            .map_err(|_| "resource manager lock poisoned")?;
        if manager.work.is_some() {
            return Err("application work depot was already configured".into());
        }
        let root = application_root.canonicalize().map_err(|e| e.to_string())?;
        let path = root.join("Userdata");
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())?;
        let path = path.canonicalize().map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("portable work depot leaves its application root".into());
        }
        //59E060: directory kind, writable1, priority0, dirty1, flags0. Its
        //5AD760 insertion/5ABE50 pass selects this actual last writable depot.
        manager.work = Some(Directory {
            id: DepotId::from_work_owner(),
            path,
            writable: 1,
            priority: 0,
        });
        Ok(())
    }
    ///The manager lock remains held across find/remove/construct/location/
    ///publish/name-copy, matching5A3250. Fields remain actual borrowed owners.
    pub fn film_depot<'a, 'source>(
        &'a mut self,
        catalog: &'a Catalog<'source>,
        overrides: NativeOverrides,
    ) -> Result<FilmDepot<'a, 'source>> {
        let manager = self
            .manager
            .lock()
            .map_err(|_| "resource manager lock poisoned")?;
        if manager.work.is_none() {
            return Err("actual writable application work depot is unbound".into());
        }
        Ok(FilmDepot {
            manager,
            maps: &mut self.maps,
            sources: &mut self.sources,
            catalog,
            overrides: Some(overrides),
        })
    }
    pub fn film_parts<'a, 'source>(
        &'a mut self,
        catalog: &'a Catalog<'source>,
        overrides: NativeOverrides,
    ) -> Result<(
        FilmDepot<'a, 'source>,
        &'a mut crate::film_process_metadata::State,
    )> {
        let manager = self
            .manager
            .lock()
            .map_err(|_| "resource manager lock poisoned")?;
        if manager.work.is_none() {
            return Err("actual writable application work depot is unbound".into());
        }
        let process = self
            .process
            .as_mut()
            .ok_or("actual same application process metadata is unbound")?;
        Ok((
            FilmDepot {
                manager,
                maps: &mut self.maps,
                sources: &mut self.sources,
                catalog,
                overrides: Some(overrides),
            },
            process,
        ))
    }
    pub fn open_film_file(&self, resource: &mut Resource) -> std::result::Result<(), OpenFailure> {
        let manager = self
            .manager
            .lock()
            .map_err(|_| OpenFailure::Required("resource manager lock poisoned".into()))?;
        let work = manager
            .work
            .as_ref()
            .ok_or_else(|| OpenFailure::Required("actual work depot absent".into()))?;
        let depots = resource.depots().map_err(OpenFailure::Required)?;
        if !depots.last().is_some_and(|d| d.same(&work.id)) {
            return Err(OpenFailure::Required(
                "film output's actual topmost depot is not the work directory".into(),
            ));
        }
        drop(depots);
        let loan = resource.metadata().map_err(OpenFailure::Required)?;
        let ids =
            String::from_utf16(loan.ids()).map_err(|e| OpenFailure::Required(e.to_string()))?;
        let relative = ids.replace('\\', "/");
        if relative.contains('\0')
            || Path::new(&relative).is_absolute()
            || Path::new(&relative).components().any(|p| {
                matches!(
                    p,
                    std::path::Component::ParentDir | std::path::Component::Prefix(_)
                )
            })
        {
            return Err(OpenFailure::Required(
                "film output outside proved relative filename domain".into(),
            ));
        }
        let path = work.path.join(&relative);
        drop(loan);
        let parent = path
            .parent()
            .ok_or_else(|| OpenFailure::Required("film output parent absent".into()))?;
        std::fs::create_dir_all(parent).map_err(OpenFailure::Io)?;
        let parent = parent.canonicalize().map_err(OpenFailure::Io)?;
        if !parent.starts_with(&work.path) {
            return Err(OpenFailure::Required(
                "film path leaves actual work depot".into(),
            ));
        }
        //Real writable/create/truncate backend for native open0x102. Windows
        //sharing behavior is a backend policy; native lock parity is unproved.
        let file = OpenOptions::new()
            .write(true)
            .read(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(OpenFailure::Io)?;
        resource
            .open_file_backend(file, &path.to_string_lossy())
            .map_err(OpenFailure::Required)
    }
}

#[cfg(test)]
mod application_data_owner_tests {
    use super::*;
    #[test]
    #[ignore="requires actual original/v14 archive backend; no native engine/GPU execution"]
    fn data_loan_retains_one_owner_on_error_and_allows_same_resource_manager_children(){
        let game=PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),
            aa_rules::sources::SourceLimits{max_member_bytes:u64::MAX}).unwrap();
        let application=Application::from_sources(sources);let mut first=None;
        let result:Result<()>=application.with_data_pre_request(|state|{
            first=Some(std::ptr::from_mut(state));
            assert!(application.with_data_pre_request(|_|Ok(())).unwrap_err().contains("already borrowed"));
            //Resource-child access uses the existing independent Manager. It
            //must not deadlock behind the Data scalar directory's loan.
            assert!(application.vworld_texture_subfolder()?.is_empty());
            Err("required reached resource child".into())
        });
        assert_eq!(result.unwrap_err(),"required reached resource child");
        application.with_data_pre_request(|state|{assert_eq!(first,Some(std::ptr::from_mut(state)));Ok(())}).unwrap();
    }
}
fn resolve_work_path(work: &Directory, ids: &[u16]) -> Result<PathBuf> {
    if work.writable <= 0 || work.priority != 0 {
        return Err("portable work directory lost its actual constructor/selection fields".into());
    }
    let ids = String::from_utf16(ids).map_err(|e| e.to_string())?;
    let relative = ids.replace('\\', "/");
    if relative.contains('\0')
        || Path::new(&relative).is_absolute()
        || Path::new(&relative).components().any(|p| {
            matches!(
                p,
                std::path::Component::ParentDir | std::path::Component::Prefix(_)
            )
        })
    {
        return Err("work filename outside proved relative domain".into());
    }
    Ok(work.path.join(relative))
}
///An immutable native filename/depot loan. It cannot outlive its actual
///application manager or be fabricated from a caller's arbitrary path.
pub struct WorkPath {
    origin: Weak<()>,
    depot: DepotId,
    root: PathBuf,
    path: PathBuf,
    ids: Vec<u16>,
}
impl WorkPath {
    pub(crate) fn require_application(&self, application: &Application) -> Result<()> {
        if self.origin.upgrade().is_none()
            || !self.origin.ptr_eq(&Arc::downgrade(&application.origin))
        {
            return Err("DataText work filename belongs to another application manager".into());
        }
        Ok(())
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn logical_ids(&self) -> &[u16] {
        &self.ids
    }
    pub fn depot_id(&self) -> &DepotId {
        &self.depot
    }
    pub fn validate_path(&self) -> Result<()> {
        if self.origin.upgrade().is_none() {
            return Err("DataText work filename's application owner was destroyed".into());
        }
        if let Some(parent) = self.path.parent() {
            if let Ok(current) = parent.canonicalize() {
                if !current.starts_with(&self.root) {
                    return Err("DataText deletion leaves actual work depot".into());
                }
            }
        }
        Ok(())
    }
    pub fn delete_existing(&self) -> Result<bool> {
        self.validate_path()?;
        //std owns the regular-file OS transport; native859B30's result is
        //reported as success/failure, without unsafe or invented OS status.
        Ok(std::fs::remove_file(&self.path).is_ok())
    }
}
pub enum OpenFailure {
    Io(std::io::Error),
    Required(String),
}
impl std::fmt::Display for OpenFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => e.fmt(f),
            Self::Required(e) => e.fmt(f),
        }
    }
}
///Exact existing allocation handle, never a filename-as-resource substitute.
pub struct Existing {
    key: String,
    resource: Resource,
}
pub struct FilmDepot<'a, 'source> {
    manager: MutexGuard<'a, Manager>,
    maps: &'a mut resource_map_cache::State,
    sources: &'a mut V14Sources,
    catalog: &'a Catalog<'source>,
    overrides: Option<NativeOverrides>,
}
impl Depot for FilmDepot<'_, '_> {
    type Existing = Existing;
    type Error = String;
    fn find(&mut self, ids: &[u16]) -> Result<Option<Existing>> {
        let key = resource_map_cache::normalized_key(ids)?;
        if let Some(resource)=self.manager.resources.get_mut(&key){resource.increment_reference14();return Err("Film lookup reached actual Texture/Model Resource; requested class differs".into());}
        if self.manager.view_resources.contains_key(&key) {return Err("Film lookup reached actual modern view/model resource; native class/refcount service required".into());}
        if self.maps.contains_key(&key) {
            self.maps.reject_wrong_type_lookup(ids)?;
            unreachable!()
        }
        if let Some(resource) = self.manager.films.get_mut(&key) {
            resource.increment_references_14_wrapping()?;
            return Ok(Some(Existing {
                key,
                resource: resource.retain_for_manager()?,
            }));
        }
        let work = self
            .manager
            .work
            .as_ref()
            .ok_or("film current work depot absent")?;
        //An uncached real disk file reaches5AEA90's load/class producer. It
        //must not be silently treated as an empty in-memory name lookup.
        if work
            .path
            .join(&key)
            .try_exists()
            .map_err(|e| e.to_string())?
        {
            return Err("existing uncached film requires actual depot5AEA90 FilmResource load/class producer".into());
        }
        Ok(None)
    }
    fn only_work_depot(&mut self, existing: &Existing) -> Result<bool> {
        let work = self.manager.work.as_ref().ok_or("film work depot absent")?;
        let depots = existing.resource.depots()?;
        Ok(depots.len() == 1 && depots[0].same(&work.id))
    }
    fn warn_existing_outside_work(&mut self, ids: &[u16]) -> Result<()> {
        self.manager.warnings.push(format!(
            "film should only exist in work depot: {}",
            String::from_utf16_lossy(ids)
        ));
        Ok(())
    }
    fn remove(&mut self, existing: Existing) -> Result<()> {
        let current = self
            .manager
            .films
            .get(&existing.key)
            .ok_or("existing film allocation disappeared")?;
        if !current.same_allocation(&existing.resource) {
            return Err("film manager allocation changed before removal".into());
        }
        //5AC510 calls76C500 over BOTH current Session LaunchState resource
        //slots before removal. Until that actual loan is installed, refuse
        //this reached positive path; do not silently clear a filename entry.
        Err(
            "film removal requires actual76C500 current Session launch-resource invalidation owner"
                .into(),
        )
    }
    fn ensure_work_depot(&mut self, resource: &mut Resource) -> Result<()> {
        let work = self.manager.work.as_ref().ok_or("film work depot absent")?;
        if resource.depots()?.iter().any(|d| d.same(&work.id)) {
            return Ok(());
        }
        if resource.requests_4()? != 0 {
            return Err(
                "5AA660 positive requested resource needs actual assertion/unload service".into(),
            );
        }
        let before = resource.base_flags_8()?;
        if before & 1 != 0 && before & 0x10 == 0 {
            return Err("5AA660 loaded resource requires actual5ABFF0 unload".into());
        }
        let id = work.id.clone();
        {
            let mut depots = resource.depots_mut()?;
            if !depots.is_empty() {
                return Err(
                    "film other-depot priority insertion requires actual foreign depot owner"
                        .into(),
                );
            }
            depots.push(id);
        }
        //Do not hold a depot-vector loan over its native virtual callback.
        resource.store_base_flags_8(before & !6)?;
        if before & 0x10 != 0 {
            resource.store_base_flags_8((before & !6) & !0x10)?;
            let key = resource_map_cache::normalized_key(resource.metadata()?.ids())?;
            if let Some(resource)=self.manager.resources.get_mut(&key){resource.increment_reference14();return Err("Film lookup reached actual Texture/Model Resource; requested class differs".into());}
        if self.manager.view_resources.contains_key(&key) {return Err("film location callback collides with actual modern view/model resource".into());}
            if self.maps.contains_key(&key) {
                return Err("film location callback collides with current MapResource".into());
            }
            self.manager
                .films
                .insert(key, resource.retain_for_manager()?);
        }
        Ok(())
    }
    fn work_is_topmost(&mut self, resource: &Resource) -> Result<bool> {
        let work = self.manager.work.as_ref().ok_or("film work depot absent")?;
        Ok(resource.depots()?.last().is_some_and(|d| d.same(&work.id)))
    }
    fn warn_work_not_topmost(&mut self, ids: &[u16]) -> Result<()> {
        self.manager.warnings.push(format!(
            "film work depot is not topmost: {}",
            String::from_utf16_lossy(ids)
        ));
        Ok(())
    }
    fn publish(&mut self, resource: &mut Resource) -> Result<()> {
        let key = resource_map_cache::normalized_key(resource.metadata()?.ids())?;
        if resource.base_flags_8()? & 0x10 != 0 {
            self.manager
                .films
                .insert(key.clone(), resource.retain_for_manager()?);
        }
        let current = self
            .manager
            .films
            .get(&key)
            .ok_or("5ACEC0 actual prior location registration absent")?;
        if !current.same_allocation(resource) {
            return Err("5ACEC0 has another native resource allocation at this IDS".into());
        }
        if self.manager.film_class.is_none() {
            let overrides = self
                .overrides
                .take()
                .ok_or("ResourceCache fresh current variable capability was consumed")?;
            self.manager.film_class = Some(
                crate::resource_cache_metadata::FilmClass::from_current_fresh_variables(
                    self.catalog,
                    self.sources,
                    overrides,
                )?,
            );
        }
        Ok(())
    }
}

#[cfg(test)]mod view_residency_tests {
    use super::*;
    fn source()->aa_rules::resources::Provenance {aa_rules::resources::Provenance {
        path:"explicit-model-input.tgi".into(),sha256:"explicit-lifetime-fixture".into(),span:aa_rules::tgi::Span {start:1,end:9}}}
    #[test]fn upload_witness_requires_same_live_application_view_and_input() {
        let application=Arc::new(());let view=Arc::new(());
        let witness=ViewResourceWitness {application:Arc::downgrade(&application),identity:Arc::downgrade(&view),
            key:resource_map_cache::normalized_key(&"Units/Actor.nif".encode_utf16().collect::<Vec<_>>()).unwrap(),definition:source()};
        assert!(witness.validate("Units/Actor.nif",&source()).is_ok());
        assert!(witness.validate("Units/Other.nif",&source()).is_err());
        let mut changed=source();changed.sha256.push('!');assert!(witness.validate("Units/Actor.nif",&changed).is_err());
        drop(view);assert!(witness.validate("Units/Actor.nif",&source()).is_err());
        let replacement=Arc::new(());
        let witness=ViewResourceWitness {identity:Arc::downgrade(&replacement),..witness};
        assert!(witness.validate("Units/Actor.nif",&source()).is_ok());
        drop(application);assert!(witness.validate("Units/Actor.nif",&source()).is_err());
    }
    #[test]fn expired_view_is_not_gpu_unload_and_changed_context_retains_assignment() {
        let view=Arc::new(());let mut resources=BTreeMap::new();
        resources.insert("actual-registered-model.nif".into(),ViewResourceRegistration {definition:source(),leases:vec![Arc::downgrade(&view)]});
        drop(view);
        let mut context=crate::vworld_resource_context::State::constructed();
        let next="new-biome/".encode_utf16().collect::<Vec<_>>();
        let error=context.assign_and_invalidate(&next,||{
            require_no_unbound_view_invalidation(&resources)?;Ok((0,0))
        }).unwrap_err();
        assert!(error.contains("5ACA50"));assert_eq!(context.current(),next);
        assert_eq!(resources.len(),1);
        assert!(!context.assign_and_invalidate(&next,||panic!("equal native prefix skips invalidation")).unwrap().changed);
    }
    ///The Firefight (Tropical) prefix: constructed/requested/loaded Models and
    ///other classes skip; only a cached unrequested Model reaches5ABFF0.
    #[test]fn namespace_5aca50_skips_uncached_models_and_refuses_cached_unload() {
        use crate::resource_objects::{Entry,SourceFile};
        let origin=Arc::new(());let priority=crate::resource_task::DefaultPriority::from_fresh_application(&origin);
        let source=|name:&str|SourceFile::from_catalog(&aa_rules::sources::CatalogEntry{logical_path:name.into(),size:0,
            origin:aa_rules::sources::Origin::OriginalArchive{member_index:0}});
        let ids=|name:&str|name.encode_utf16().collect::<Vec<_>>();
        let mut resources=BTreeMap::new();
        let mut model=Entry::model(&origin,ids("Units/a.nif"),source("Units/a.nif"),&priority).unwrap();
        model.attach_selected_source(source("Units/a.nif")).unwrap();
        resources.insert("units/a.nif".to_string(),model);
        let mut requested=Entry::model(&origin,ids("Units/b.nif"),source("Units/b.nif"),&priority).unwrap();
        requested.requests4=1;requested.flags8|=7;
        resources.insert("units/b.nif".to_string(),requested);
        let mut texture=Entry::texture(&origin,ids("Art/c.dds"),source("Art/c.dds"),&priority).unwrap();
        texture.flags8|=0x21;
        resources.insert("art/c.dds".to_string(),texture);
        let maps=resource_map_cache::State::constructed();let films=BTreeMap::new();let views=BTreeMap::new();
        let mut context=crate::vworld_resource_context::State::constructed();
        let tropical="Tropical".encode_utf16().collect::<Vec<_>>();
        let update=context.assign_and_invalidate(&tropical,||invalidate_namespace_5aca50(&maps,&films,&resources,&views)).unwrap();
        assert!(update.changed);assert_eq!((update.maps_examined,update.films_examined),(0,0));
        assert_eq!(resources["units/a.nif"].flags8,0x400);assert_eq!(resources["units/b.nif"].requests4,1);
        //A released+cached entry (bit5, count0) cannot be produced by the
        //port yet; when it exists the native unload is required, +80 kept.
        resources.get_mut("units/b.nif").unwrap().requests4=0;resources.get_mut("units/b.nif").unwrap().flags8|=0x20;
        let arctic="Arctic".encode_utf16().collect::<Vec<_>>();
        let error=context.assign_and_invalidate(&arctic,||invalidate_namespace_5aca50(&maps,&films,&resources,&views)).unwrap_err();
        assert!(error.contains("5ABFF0")&&error.contains("Units/b.nif"),"{error}");assert_eq!(context.current(),arctic);
    }
}

///A scoped loan of the actual generic ResourceManager/source backend. This
///contains no copied mutable fields and creates no second manager or namespace.
pub struct Resources<'a>{
    origin:&'a Arc<()>,sources:&'a mut V14Sources,maps:&'a mut resource_map_cache::State,
    manager:&'a Mutex<Manager>,priority:&'a crate::resource_task::DefaultPriority,tasks:&'a crate::resource_task::TaskQueue,
    cache_variables:&'a Option<crate::resource_audio_class::FreshCacheVariableScope>,
    model_source_context:&'a Option<crate::resource_model_source::Context>,
}
enum CachedResource {Concrete(crate::resource_objects::Handle),Map,Film}
impl Resources<'_>{
    pub(crate) fn application_witness(&self)->Weak<()>{Arc::downgrade(self.origin)}
    pub(crate) fn require_data_application(&self,owner:&Weak<()>)->Result<()>{
        if owner.upgrade().is_none()||!Weak::ptr_eq(owner,&Arc::downgrade(self.origin)){return Err("DataFXSession Resource Application differs or is retired".into())}Ok(())
    }
    fn source(&self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank)->Result<()>{
        catalog.require_source_owner()?;
        let fields=bank.combat_source()?;fields.completed_means()?;
        if !crate::catalog_lookups::same_sources(catalog,fields.source_identities()){return Err("Resource factory actual Bank/source owner differs".into());}
        Ok(())
    }
    ///5AD430/5AEA90 cached part; every successful lookup increments actual14,
    ///including a later wrong-class result. No hash traversal order escapes.
    fn cached(&mut self,ids:&[u16])->Result<Option<CachedResource>>{
        let key=resource_map_cache::normalized_key(ids)?;
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        if let Some(entry)=manager.resources.get_mut(&key){entry.increment_reference14();return Ok(Some(CachedResource::Concrete(entry.handle.clone())));}
        if self.maps.contains_key(&key){self.maps.increment_cached_reference_14(&key)?;return Ok(Some(CachedResource::Map));}
        if let Some(film)=manager.films.get_mut(&key){film.increment_references_14_wrapping()?;return Ok(Some(CachedResource::Film));}
        if manager.view_resources.contains_key(&key){return Err("native Resource lookup reached modern residency without actual Resource class/refcount owner".into());}
        Ok(None)
    }
    fn find_typed(&mut self,ids:&[u16],expected:crate::resource_objects::Kind)->Result<crate::resource_objects::Handle>{
        self.find_typed_optional(ids,expected)?.ok_or_else(||format!("5130A0 actual requested Resource source file is missing: {}",String::from_utf16_lossy(ids)))
    }
    fn find_typed_optional(&mut self,ids:&[u16],expected:crate::resource_objects::Kind)->Result<Option<crate::resource_objects::Handle>>{
        use crate::resource_objects::{Entry,Kind,SourceFile};
        if let Some(hit)=self.cached(ids)?{
            return match hit{CachedResource::Concrete(v)if v.kind()==expected=>Ok(Some(v)),_=>Err("cached allocation has wrong Resource class; native14 increment retained".into())};
        }
        let key=resource_map_cache::normalized_key(ids)?;
        {
            let manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
            if let Some(work)=&manager.work{
                let path=resolve_work_path(work,ids)?;
                if path.try_exists().map_err(|e|e.to_string())?{return Err("5AEA90 actual work-directory Resource requires retained writable depot/class loader".into());}
            }
        }
        let matches=self.sources.rows_with_lookup_key(&key);
        if matches.is_empty(){return Ok(None)}
        let [selected]=matches.as_slice()else{return Err("actual accepted Resource source file is ambiguous".into())};
        let kind=crate::resource_objects::class_for_name(ids)?.ok_or("5AEA90 actual source class constructor remains required")?;
        let source=SourceFile::from_catalog(selected);
        let resource_ids=ids.iter().copied().take_while(|&v|v!=0).map(|v|if v==92{47}else{v}).collect();
        let mut entry=match kind{Kind::Texture=>Entry::texture(self.origin,resource_ids,source.clone(),self.priority)?,Kind::Model=>Entry::model(self.origin,resource_ids,source.clone(),self.priority)?,Kind::Sequence=>Entry::sequence(self.origin,resource_ids,source.clone(),self.priority)?,Kind::Sound=>Entry::sound(self.origin,resource_ids,source.clone(),self.priority)?};
        entry.attach_selected_source(source)?;
        let handle=entry.handle.clone();
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        if manager.resources.contains_key(&key)||manager.films.contains_key(&key)||self.maps.contains_key(&key){return Err("Resource namespace changed during exclusive source constructor loan".into());}
        manager.resources.insert(key,entry);
        if kind!=expected{return Err("new actual Resource has wrong requested class; same published allocation retained".into());}
        Ok(Some(handle))
    }
    ///5A77E0: the biome candidate is a CURRENT cached-resource probe, not a
    ///filesystem existence probe. A candidate hit incurs both native14 reads.
    pub fn texture_5a77e0(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16],flags:u32)->Result<Option<crate::resource_objects::Handle>>{
        self.texture_5a77e0_with_policy(catalog,bank,ids,flags,&mut crate::resource_objects::RequiredTexturePolicy)
    }
    ///Only the final original-name lookup lies in the native try block
    ///(FuncInfoAEECBC state6). Wrong-class and candidate-lookup errors escape.
    ///Genuine accepted-depot absence is the only caught outcome represented
    ///here; loader failures never become a soft miss by string matching.
    pub fn texture_5a77e0_with_policy(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16],flags:u32,policy:&mut impl crate::resource_objects::TexturePolicy)->Result<Option<crate::resource_objects::Handle>>{
        self.source(catalog,bank)?;
        let subfolder=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.texture_subfolder_80.current().to_vec();
        if !subfolder.is_empty(){
            let candidate=crate::resource_objects::subfolder_candidate(ids,&subfolder)?;
            if self.cached(&candidate)?.is_some(){return self.find_typed(&candidate,crate::resource_objects::Kind::Texture).map(Some);}
        }
        if let Some(handle)=self.find_typed_optional(ids,crate::resource_objects::Kind::Texture)?{return Ok(Some(handle))}
        match crate::resource_objects::missing_texture_5a7976(flags,policy)?{
            crate::resource_objects::MissingTexture::Null=>Ok(None),
            crate::resource_objects::MissingTexture::Rethrow=>Err("5130A0 missing Texture source; native DataCompatibilityMode is zero and rethrows".into()),
            crate::resource_objects::MissingTexture::DefaultRequired=>self.default_texture_5a6e90(catalog,ids,policy).map(Some),
        }
    }
    fn default_texture_5a6e90(&mut self,catalog:&Catalog<'_>,ids:&[u16],policy:&mut impl crate::resource_objects::TexturePolicy)->Result<crate::resource_objects::Handle>{
        let warning=policy.data_missing_resource_warning_current()?;
        policy.missing_texture_diagnostic_404fc0(ids,crate::resource_objects::missing_texture_diagnostic_level(warning))?;
        let key=resource_map_cache::normalized_key(ids)?;
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let work=manager.work.as_ref().filter(|d|d.writable>0).ok_or("5A6E90 requires actual selected writable depot90")?;
        let resource_ids=ids.iter().copied().take_while(|&v|v!=0).map(|v|if v==92{47}else{v}).collect();
        let entry=crate::resource_objects::Entry::default_texture(self.origin,resource_ids,self.priority,work.id.clone())?;
        let handle=entry.handle.clone();
        if manager.resources.contains_key(&key)||manager.films.contains_key(&key)||self.maps.contains_key(&key)||manager.view_resources.contains_key(&key){return Err("default Texture publication reached a changed actual Resource namespace".into())}
        manager.resources.insert(key.clone(),entry);
        //5ACEC0 has already published the attached allocation when59CB00
        //reaches its class-current lookup. An error retains that ref14=0
        //prefix in this same manager; the following increment has not run.
        if manager.texture_class.is_none(){
            let scope=policy.texture_cache_source_overrides()?;
            manager.texture_class=Some(crate::resource_texture_class::TextureClass::from_current_fresh_variables(catalog,self.sources,scope)?);
        }
        manager.resources.get_mut(&key).expect("same just-published allocation").increment_reference14();
        Ok(handle)
    }
    ///5A8520 preserves BOTH current73A560 calls where the candidate misses.
    ///The bool is supplied by the actual client renderer capability; no raw
    ///hardware default is substituted by this runtime/source factory.
    pub fn model_5a8520(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16],policy:&mut impl crate::resource_objects::ModelPolicy)->Result<crate::resource_objects::Handle>{
        self.source(catalog,bank)?;
        let subfolder=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.texture_subfolder_80.current().to_vec();
        if !subfolder.is_empty(){
            let candidate=crate::resource_objects::subfolder_candidate(ids,&subfolder)?;
            let variant=crate::resource_objects::current_model_variant(&candidate,policy)?;
            if self.cached(&variant)?.is_some(){let h=self.find_typed(&variant,crate::resource_objects::Kind::Model)?;self.require_model_class_accounting(catalog,bank,&h)?;return Ok(h);}
            if self.cached(&candidate)?.is_some(){let h=self.find_typed(&candidate,crate::resource_objects::Kind::Model)?;self.require_model_class_accounting(catalog,bank,&h)?;return Ok(h);}
        }
        let variant=crate::resource_objects::current_model_variant(ids,policy)?;
        if self.cached(&variant)?.is_some(){let h=self.find_typed(&variant,crate::resource_objects::Kind::Model)?;self.require_model_class_accounting(catalog,bank,&h)?;return Ok(h);}
        let h=self.find_typed(ids,crate::resource_objects::Kind::Model)?;self.require_model_class_accounting(catalog,bank,&h)?;Ok(h)
    }
    fn require_model_class_accounting(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,handle:&crate::resource_objects::Handle)->Result<()>{
        handle.require_application(self.origin)?;let key=resource_map_cache::normalized_key(handle.logical_ids())?;
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let entry=manager.resources.get(&key).filter(|e|e.handle.same_identity(handle)).ok_or("59CB00 current Model allocation differs")?;
        if entry.model.is_none(){return Err("59CB00 requires actual Model allocation".into())}
        if manager.model_class.is_none(){let scope=self.cache_variables.as_ref().ok_or("59CB00 actual fresh Model cache-variable policy absent")?;
            manager.model_class=Some(crate::resource_model_class::ModelClass::from_current(catalog,bank,self.sources,scope)?);}
        Ok(())
    }
    ///454F70 reads actual Resource+38 after469FE0; this is the retained
    ///constructor CString, never another source Find or texture-biome resolve.
    pub fn model_filename_38(&self,handle:&crate::resource_objects::Handle)->Result<Vec<u16>>{
        self.with_entry(handle,|v|{
            if v.model.is_none(){return Err("Model filename38 requires actual Model allocation".into());}
            Ok(v.handle.logical_ids().to_vec())
        })
    }
    ///457A40 reads this SAME model field only after its current flags8bit2.
    pub fn model_radius_64(&self,handle:&crate::resource_objects::Handle)->Result<u32>{self.with_entry(handle,|v|v.model.as_ref().map(|v|v.radius64).ok_or_else(||"Resource is not actual Model allocation".into()))}
    pub fn model_payload_40(&self,handle:&crate::resource_objects::Handle)->Result<Option<crate::resource_model_payload::Payload>>{self.with_entry(handle,|e|{let payload=e.model.as_ref().ok_or("actual Model allocation required")?.source40.clone();if let Some(payload)=&payload{payload.require_resource(handle)?}Ok(payload)})}
    ///5A63D0's current source pointer. Accepted physical-source allocations
    ///have a read-only selected depot; positive writable-child delegation must
    ///be bound before such a depot can enter this manager's resource directory.
    pub fn texture_5a63d0(&self,handle:&crate::resource_objects::Handle)->Result<Option<crate::resource_texture::SourceTexture>>{
        self.with_entry(handle,|v|v.texture.as_ref().map(|v|v.source3c.clone()).ok_or_else(||"GetTexture requires actual Texture Resource allocation".into()))
    }
    ///42CD60's current three operations on the SAME Resource pointer. The
    ///render-format copy precedes reading/rendering flags, including failures.
    pub fn radar_texture_policy_42cd60(&self,handle:&crate::resource_objects::Handle,policy:&mut impl crate::resource_objects::TexturePolicy)->Result<()>{
        let format=self.with_texture_objects(|owner|Ok(owner.default_format_b43d2c()))?;
        self.with_entry(handle,|e|{
            let t=e.texture.as_mut().ok_or("5A6D10 requires actual Texture allocation")?;
            crate::resource_texture_settings::push_5a6c70(&mut t.render_slots,&mut t.render_count60,format)
        })?;
        let reading=self.texture_settings_target(handle)?;
        self.with_entry(&reading,|entry|crate::resource_texture_settings::set_reading_at(&mut TextureSettingsEntry{entry,policy},(),1))?;
        //5A6340 independently traverses the current last-location/child chain.
        let rendering=self.texture_settings_target(handle)?;
        self.with_entry(&rendering,|entry|crate::resource_texture_settings::set_rendering_at(&mut TextureSettingsEntry{entry,policy},(),0))
    }
    fn texture_settings_target(&self,handle:&crate::resource_objects::Handle)->Result<crate::resource_objects::Handle>{
        let mut current=handle.clone();
        while self.texture_last_location_writable(&current)?.unwrap_or(false){
            let child=self.with_entry(&current,|e|Ok(e.texture.as_ref().ok_or("Texture options require actual Texture")?.writable_child40.clone()))?;
            let Some(child)=child else{break};current=child;
        }
        Ok(current)
    }
    ///49DA40's source LoadData boundary5A80D0→5A7750(flags1). Unlike Model
    ///and Texture factories it does not consult current biome subfolder80 or
    ///a rendering policy. The SAME manager publishes/returns its typed KF
    ///allocation; decoder/request completion remains a later real operation.
    ///Without a current-variable policy a genuinely absent source cannot take
    ///5A80D0's catch path; use [`Self::sequence_5a80d0_with_policy`].
    pub fn sequence_5a80d0(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16])->Result<crate::resource_objects::Handle>{
        self.source(catalog,bank)?;
        self.find_typed_optional(ids,crate::resource_objects::Kind::Sequence)?.ok_or_else(||format!(
            "5130A0 actual requested Resource source file is missing: {}; 5A80D0 catches this TGC_MissingResourceException and needs current UVar61 DataCompatibilityMode/DataMissingResourceWarning (sequence_5a80d0_with_policy)",
            String::from_utf16_lossy(ids)))
    }
    ///KKC_NiSeqResource::GetNiSeqResource 5A80D0, the only Sequence factory
    ///of Animation463B50 and ObjectAnimation49DA40 (neither has a try block).
    ///It calls5A7750(name,1) in try state0 (FuncInfo AEED7C) with ONE catch,
    ///`TGC_MissingResourceException&` (B28008) at5A8115. 5A7750 throws that
    ///type through425210 only when5AD430 and5AEA90 both find nothing; its
    ///wrong-class4252A0 throws TGC_CorruptResourceException, which escapes.
    ///The handler reads byte [B39940]+F4 (UVar61 DataCompatibilityMode, the
    ///SAME cell5A7976 reads). Zero rethrows (96BEF5(0,0)). Otherwise405D20
    ///reads B3994C "DataMissingResourceWarning" (the SAME lookup5A6E90
    ///inlines), calls404FC0(B38C28,level,"Missing animation resource '%s'
    ///- disabling animation") and returns NULL (5A816E `xor eax,eax`).
    ///463B50 then constructs the Animation with that NULL sequence; 49DA40
    ///skips its ObjectAnimation allocation. The policy supplies only the two
    ///current variable reads; nothing is published for the missing name.
    pub fn sequence_5a80d0_with_policy(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16],
        policy:&mut impl crate::resource_objects::TexturePolicy)->Result<Option<crate::resource_objects::Handle>>{
        self.source(catalog,bank)?;
        if let Some(handle)=self.find_typed_optional(ids,crate::resource_objects::Kind::Sequence)?{return Ok(Some(handle))}
        if policy.data_compatibility_mode_current()?==0{
            return Err(format!("5130A0 missing Sequence source {}; native DataCompatibilityMode is zero and 5A80D0 rethrows",String::from_utf16_lossy(ids)));
        }
        let level=missing_resource_level(policy.data_missing_resource_warning_current()?);
        self.record_missing_diagnostic(MissingResourceDiagnostic{channel:DiagnosticChannel::ResourceB38C28,level,
            file:".\\Source\\resource\\NiResource.cpp",line:0x6c6,function:"KKC_NiSeqResource::GetNiSeqResource",
            format:"Missing animation resource '%s' - disabling animation",ids:ids.to_vec()})?;
        Ok(None)
    }
    fn record_missing_diagnostic(&self,diagnostic:MissingResourceDiagnostic)->Result<()>{
        self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.missing_resource_diagnostics.push(diagnostic);Ok(())
    }
    ///The SAME manager's caught missing-source404FC0 records, in call order.
    pub fn missing_resource_diagnostics(&self)->Result<Vec<MissingResourceDiagnostic>>{
        Ok(self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.missing_resource_diagnostics.clone())
    }
    ///59B1F0(hard1), then59B4B0: a category1/2 request writes streaming41
    ///only after the SAME generic cached/source allocation passes its type
    ///test. A cache wrong-class failure retains its preceding14 increment.
    ///Without a current-variable policy a genuinely absent source cannot take
    ///59B4B0's catch path; use [`Self::sound_59b4b0_with_policy`].
    pub fn sound_59b4b0(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16],streaming:bool)->Result<crate::resource_objects::Handle>{
        self.source(catalog,bank)?;
        let handle=self.find_typed_optional(ids,crate::resource_objects::Kind::Sound)?.ok_or_else(||format!(
            "5130A0 actual requested Resource source file is missing: {}; 59B4B0 catches this TGC_MissingResourceException and needs current UVar61 DataCompatibilityMode/DataMissingResourceWarning (sound_59b4b0_with_policy)",
            String::from_utf16_lossy(ids)))?;
        self.finish_sound_59b4b0(catalog,bank,handle,streaming)
    }
    fn finish_sound_59b4b0(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,handle:crate::resource_objects::Handle,streaming:bool)->Result<crate::resource_objects::Handle>{
        //59CB00 follows the SAME Entry/task namespace publication. A class
        //source refusal retains that prefix before streaming41/list append.
        self.require_sound_class_accounting(catalog,bank,&handle)?;
        if streaming{self.with_entry(&handle,|entry|{
            entry.sound.as_mut().ok_or("59B4B0 actual Sound allocation is required")?.streaming41=1;Ok(())
        })?;}
        Ok(handle)
    }
    ///59B4B0 with its actual catch. It calls59B1F0(name,1) in try state0
    ///(FuncInfo AED848) with ONE catch, `TGC_MissingResourceException&`
    ///(B28008) at59B502; 46BB60, its only caller, has no try block. The
    ///handler reads byte [B39940]+F4 (UVar61 DataCompatibilityMode, the SAME
    ///cell5A7976 reads). Zero rethrows (96BEF5(0,0)). Otherwise it returns
    ///59B280(name,streaming) = KKC_AudioResource::GetDefaultResource:
    ///405370 on B3994C "DataMissingResourceWarning" (the SAME lookup5A6E90
    ///inlines), 404FC0(B38B98,level,"Missing audio resource '%s' - replacing
    ///with default"), then410CB0 on B39988 reads the string AVar
    ///DefaultAudioResource (DefaultAudioStreamingResource when streaming) and
    ///59B1F0(that name,1) finds it. That inner find runs inside the handler,
    ///so ITS absence escapes uncaught. Streaming stores +41=1 on the default.
    ///The missing name itself is never published.
    pub fn sound_59b4b0_with_policy(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,ids:&[u16],streaming:bool,
        policy:&mut impl crate::resource_objects::TexturePolicy)->Result<crate::resource_objects::Handle>{
        self.source(catalog,bank)?;
        if let Some(handle)=self.find_typed_optional(ids,crate::resource_objects::Kind::Sound)?{
            return self.finish_sound_59b4b0(catalog,bank,handle,streaming)
        }
        if policy.data_compatibility_mode_current()?==0{
            return Err(format!("5130A0 missing Sound source {}; native DataCompatibilityMode is zero and 59B4B0 rethrows",String::from_utf16_lossy(ids)));
        }
        let level=missing_resource_level(policy.data_missing_resource_warning_current()?);
        self.record_missing_diagnostic(MissingResourceDiagnostic{channel:DiagnosticChannel::AudioB38B98,level,
            file:".\\Source\\resource\\AudioResource.cpp",line:0x3e,function:"KKC_AudioResource::GetDefaultResource",
            format:"Missing audio resource '%s' - replacing with default",ids:ids.to_vec()})?;
        let name=self.default_audio_resource_410cb0(catalog,bank,streaming)?;
        let handle=self.find_typed_optional(&name,crate::resource_objects::Kind::Sound)?.ok_or_else(||format!(
            "59B280 default Sound source {} is missing; its 59B1F0 throw inside the 59B4B0 handler is uncaught",String::from_utf16_lossy(&name)))?;
        self.finish_sound_59b4b0(catalog,bank,handle,streaming)
    }
    ///410CB0 on B39988 for the AVar string59B280 names. Read from the
    ///canonical AVars* declarations (Settings.app_variables, the same source
    ///as language_ids) only inside the installed fresh no-profile variable
    ///window; a second assignment would need the native override order.
    fn default_audio_resource_410cb0(&self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,streaming:bool)->Result<Vec<u16>>{
        let scope=self.cache_variables.as_ref().ok_or("59B280 410CB0 requires the installed fresh no-profile variable window")?;
        scope.require(catalog,bank)?;
        let name=if streaming{"DefaultAudioStreamingResource"}else{"DefaultAudioResource"};
        let settings=catalog.settings.as_ref().ok_or("59B280 410CB0 public AVars Settings owner absent")?;
        let rows=settings.app_variables.get(&name.to_ascii_lowercase()).ok_or_else(||format!("59B280 410CB0 could not find var '{name}'"))?;
        let [row]=rows.as_slice() else{return Err(format!("59B280 410CB0 {name} has several AVars assignments; native override order is required"))};
        if !row.field_type.as_deref().is_some_and(|t|t.eq_ignore_ascii_case("string")){return Err(format!("59B280 410CB0 {name} requires a native String declaration"))}
        if !scope.variable_files().iter().any(|v|v.path==row.source.path&&v.sha256==row.source.sha256){
            return Err(format!("59B280 410CB0 {name} declaration lies outside the fresh variable window"))
        }
        //5BDF80: only a value whose first character is '"' is quoted, and
        //the quote ends at the next '"'.
        let value=match row.raw.strip_prefix('"'){Some(rest)=>rest.split('"').next().unwrap_or_default(),None=>row.raw.as_str()};
        if value.is_empty()||!value.is_ascii()||value.contains('\0'){return Err(format!("59B280 410CB0 {name} value lies outside the proved plain ASCII name domain"))}
        Ok(value.encode_utf16().collect())
    }
    fn require_sound_class_accounting(&mut self,catalog:&Catalog<'_>,bank:&crate::definitions::Bank,handle:&crate::resource_objects::Handle)->Result<()>{
        handle.require_application(self.origin)?;
        let key=resource_map_cache::normalized_key(handle.logical_ids())?;
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let entry=manager.resources.get(&key).filter(|e|e.handle.same_identity(handle)).ok_or("59CB00 current Sound allocation differs")?;
        if entry.sound.is_none(){return Err("59CB00 requires actual Sound allocation".into())}
        if manager.audio_class.is_none(){
            let scope=self.cache_variables.as_ref().ok_or("59CB00 actual fresh cache-variable policy owner is absent")?;
            manager.audio_class=Some(crate::resource_audio_class::AudioClass::from_current(catalog,bank,self.sources,scope)?);
        }
        Ok(())
    }
    ///46C3A0 writes this actual shared Sound byte after all three source
    ///lists load successfully and only when ignore_rate is false.
    pub fn sound_standard_rate_40(&self,handle:&crate::resource_objects::Handle)->Result<()>{
        self.with_entry(handle,|entry|{entry.sound.as_mut().ok_or("46C3A0 actual Sound allocation is required")?.standard_rate40=1;Ok(())})
    }
    pub fn sound_bytes_40_42(&self,handle:&crate::resource_objects::Handle)->Result<[u8;3]>{
        self.with_entry(handle,|entry|{let sound=entry.sound.as_ref().ok_or("actual Sound allocation is required")?;Ok([sound.standard_rate40,sound.streaming41,sound.byte42])})
    }
    ///469FE0's exact two authored flags. This API cannot mark native loaded/
    ///bounds-ready bits or manufacture a decoded model radius.
    pub fn aspect_model_flags(&self,handle:&crate::resource_objects::Handle,unique:bool,has_skin:bool)->Result<()>{self.with_entry(handle,|v|{
        if v.model.is_none(){return Err("Aspect flags require actual Model resource".into());}
        if unique{v.flags8|=0x80000;}else if has_skin{v.flags8|=0x8000;}Ok(())
    })}
    ///49B9F0/49B970 source-request options only. The accepted bits cannot mark
    ///native loaded/bounds fields or replace the same Model allocation.
    pub fn data_model_flags(&self,handle:&crate::resource_objects::Handle,flags:u32)->Result<()>{
        if flags&!0x1900!=0{return Err("Data Model flags contain an unproved option bit".into())}
        self.with_entry(handle,|v|{if v.model.is_none(){return Err("Data flags require actual Model resource".into())}v.flags8|=flags;Ok(())})
    }
    ///495F10 writes after the actual Data40 pointer store. These two flags
    ///are unrelated to native loaded/bounds-ready bits; their stores remain
    ///on this SAME allocation and cannot be replaced by a GPU receipt.
    pub fn vfx_model_flags(&self,handle:&crate::resource_objects::Handle,floating_ui:bool)->Result<()>{self.with_entry(handle,|v|{
        if v.model.is_none(){return Err("VFX model flags require actual Model Resource".into());}
        if floating_ui{v.flags8|=0x80000;}
        v.flags8&=!0x400;Ok(())
    })}
    ///5A55C0 changes a pre-load option, never native ready/bounds fields.
    pub fn set_load_for_reading_5a55c0(&self,handle:&crate::resource_objects::Handle,value:bool)->Result<()>{self.with_entry(handle,|v|{
        if v.model.is_none(){return Err("SetLoadForReading requires actual Model Resource".into());}
        if v.flags8&1!=0{return Err("5A55C0 loaded model warning404ED0 requires actual diagnostic sink".into());}
        if value{v.flags8|=0x100000;}else{v.flags8&=!0x100000;}Ok(())
    })}
    ///Resource vf10=5A6190: only the LOW mode byte selects immediate4;
    ///otherwise read the SAME current ApplicationB282F0 cell lazily.
    pub fn request_10(&mut self,handle:&crate::resource_objects::Handle,delta:i32,mode:u32,backend:&mut impl Backend)->Result<()>{
        let priority=if mode as u8!=0{4}else{self.priority.current()};
        self.request_0c(handle,delta,priority,backend)
    }
    pub fn request_0c(&mut self,handle:&crate::resource_objects::Handle,delta:i32,mode:u32,backend:&mut impl Backend)->Result<()>{
        //Texture's positive writable child would be a DIFFERENT actual
        //allocation. No source-loaded child may be guessed from location data.
        let (count,task)=self.with_entry(handle,|v|{
            if v.texture.as_ref().is_some_and(|v|v.writable_child40.is_some()){
                return Err("5A62B0 actual writable child dispatch remains required".into());
            }
            v.requests4=v.requests4.wrapping_add(delta as u32);
            Ok((v.requests4 as i32,Arc::clone(&v.task)))
        })?;
        let queue=self.tasks;
        let mut service=ResourceCallbacks{resources:self,backend};
        if delta==1{
            if count>1{if mode==4{return block_resource(&task,queue,&mut service)}return Ok(());}
            crate::resource_task::Services::enter_request(&mut service,handle)?;
            let check=(||->Result<bool>{
                let flags=crate::resource_task::Services::resource_flags8(&mut service,handle)?;
                if flags&0x20==0&&flags&1!=0||task.queued10()?{
                    service.backend.double_request_warning(handle,count)?;
                    if mode==4{block_resource(&task,queue,&mut service)?;}Ok(true)
                }else{Ok(false)}
            })();
            let cleanup=crate::resource_task::Services::leave_request(&mut service,handle);
            let skip=check?;cleanup?;if skip{return Ok(())}
        }else if count>delta{if mode==4{return block_resource(&task,queue,&mut service)}return Ok(());}
        task.request_5b0270(queue,mode,&mut service)
    }
    pub fn block_18(&mut self,handle:&crate::resource_objects::Handle,backend:&mut impl Backend)->Result<()>{
        let task=self.with_entry(handle,|v|Ok(Arc::clone(&v.task)))?;let queue=self.tasks;
        block_resource(&task,queue,&mut ResourceCallbacks{resources:self,backend})
    }
    pub(crate) fn with_entry<R>(&self,handle:&crate::resource_objects::Handle,call:impl FnOnce(&mut crate::resource_objects::Entry)->Result<R>)->Result<R>{
        handle.require_application(self.origin)?;
        let key=resource_map_cache::normalized_key(handle.logical_ids())?;
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let entry=manager.resources.get_mut(&key).ok_or("actual Resource allocation no longer registered")?;
        if !entry.handle.same_identity(handle){return Err("Resource handle generation/allocation differs".into());}
        call(entry)
    }
    pub(crate) fn with_texture_objects<R>(&self,call:impl FnOnce(&mut crate::resource_texture_owner::State)->Result<R>)->Result<R>{
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        call(&mut manager.texture_objects)
    }
    pub(crate) fn texture_last_location_writable(&self,handle:&crate::resource_objects::Handle)->Result<Option<bool>>{
        handle.require_application(self.origin)?;
        let key=resource_map_cache::normalized_key(handle.logical_ids())?;
        let manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let entry=manager.resources.get(&key).filter(|v|v.handle.same_identity(handle)).ok_or("Texture actual Resource allocation differs")?;
        match entry.locations.last(){
            None=>Ok(None),Some(crate::resource_objects::Location::Physical(_))=>Ok(Some(false)),
            Some(crate::resource_objects::Location::Work(id))=>{
                let work=manager.work.as_ref().filter(|v|v.id.same(id)).ok_or("Texture location refers to an unavailable actual writable depot")?;
                Ok(Some(work.writable>0))
            }
        }
    }
    ///The concrete5A8CD0 call mutates the same Entry and Source/Pixel arena.
    ///A failed call remains retained; retry cannot erase native prefixes.
    pub fn load_texture_4c(&mut self,handle:&crate::resource_objects::Handle,pc:aa_numeric::Precision,backend:&mut impl crate::texture_resource_load::Backend)->Result<bool>{
        crate::texture_resource_load::load(self,handle,pc,backend)
    }
    pub fn load_model_4c(&mut self,handle:&crate::resource_objects::Handle,pc:aa_numeric::Precision,backend:&mut impl crate::model_resource_load::Backend)->Result<bool>{crate::model_resource_load::load(self,handle,pc,backend)}
    pub(crate) fn model_loading_context_5a99c0(&self,remove_base:bool)->Result<()>{self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.ni_cache.loading_context(remove_base)}
    pub(crate) fn model_begin_load_5a9a83(&self)->Result<()>{self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.ni_cache.begin_load();Ok(())}
    pub(crate) fn model_profile_enabled(&self)->Result<bool>{self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.ni_cache.profile_enabled()}
    ///The baseOR6 stores are performed by ResourceCallbacks before this actual
    ///5A5410 suffix. The Ni cache entry is distinct from its root payload40.
    pub fn ni_after_base_3c_5a5410(&self,handle:&crate::resource_objects::Handle,pc:aa_numeric::Precision)->Result<()>{
        handle.require_application(self.origin)?;let key=resource_map_cache::normalized_key(handle.logical_ids())?;
        let mut manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let e=manager.resources.get(&key).filter(|e|e.handle.same_identity(handle)).ok_or("5A5410 actual Resource differs")?;
        if e.flags8&0x100!=0{return Ok(())}
        let size=crate::resource_ni_cache::weighted_size(e.memory0c,pc)?;
        let root=match handle.kind(){crate::resource_objects::Kind::Model=>e.model.as_ref().ok_or("5A5410 Model fields absent")?.source40.clone(),crate::resource_objects::Kind::Sequence=>{if e.sequence.as_ref().ok_or("5A5410 Sequence fields absent")?.source40.is_some(){return Err("5A5410 actual Sequence root owner is required".into())}None},_=>return Err("5A5410 actual Ni class required".into())};
        let entry=manager.ni_cache.append(handle,root,size)?;manager.ni_cache.validate(&entry,handle)?;
        let e=manager.resources.get_mut(&key).expect("same cache-locked Resource");
        if let Some(model)=&mut e.model{model.source3c=Some(entry)}else if let Some(sequence)=&mut e.sequence{sequence.source3c=Some(entry)}else{return Err("5A5410 Ni fields changed".into())}Ok(())
    }
    pub fn texture_load_stage(&self,handle:&crate::resource_objects::Handle)->Result<Option<crate::texture_load_kernel::Stage>>{
        self.with_entry(handle,|entry|Ok(entry.texture.as_ref().ok_or("Texture load stage requires actual Texture allocation")?.load_call.as_ref().map(|v|v.state.stage())))
    }
    pub fn flags_08(&self,handle:&crate::resource_objects::Handle)->Result<u32>{self.with_entry(handle,|v|Ok(v.flags8))}
    pub fn requests_04(&self,handle:&crate::resource_objects::Handle)->Result<u32>{self.with_entry(handle,|v|Ok(v.requests4))}
    pub fn references_14(&self,handle:&crate::resource_objects::Handle)->Result<u32>{self.with_entry(handle,|v|Ok(v.references14))}
    ///49B800 reads this SAME process word separately before each nonnull
    ///ObjectAnimation Resource request. It is not a caller-chosen mode.
    pub fn current_priority_b282f0(&self)->u32{self.priority.current()}
    ///Actual backing bytes are read only at a reached resource load, never as
    ///a substitute for native flags/task completion or a converted asset path.
    pub fn read_source(&mut self,handle:&crate::resource_objects::Handle)->Result<aa_rules::sources::ResolvedFile>{
        self.with_entry(handle,|_|Ok(()))?;
        let source=handle.source_file().ok_or("generated default Texture has no physical source file; native5A4D80 is required")?;
        let input=self.sources.read(source.logical_path()).map_err(|e|e.to_string())?;
        if input.logical_path!=source.logical_path()||input.origin!=*source.origin()||input.bytes.len()as u64!=source.size(){return Err("Resource same-depot source changed at actual load".into());}
        Ok(input)
    }
    ///Same selected Model allocation and physical bytes at the reached vf4C.
    ///A prepared payload still cannot publish loaded flags or task completion.
    pub fn model_input_4c(&mut self,handle:&crate::resource_objects::Handle)->Result<crate::resource_model_payload::Input>{
        if handle.kind()!=crate::resource_objects::Kind::Model{return Err("Model vf4C input requires actual Model Resource".into())}
        let file=self.read_source(handle)?;
        let source=handle.source_file().ok_or("Model selected source absent")?.clone();
        if let Some(context)=self.model_source_context{context.require_application(self.origin)?;}
        Ok(crate::resource_model_payload::Input::new(handle.clone(),source,file.bytes,file.sha256,self.model_source_context.clone()))
    }
    ///Exact selected input loan at a reached Texture vf4C. The returned owner
    ///contains real V14Sources bytes and cannot mark the resource/task ready.
    pub fn texture_input_4c(&mut self,handle:&crate::resource_objects::Handle)->Result<crate::resource_texture::TextureInput>{
        if handle.kind()!=crate::resource_objects::Kind::Texture{return Err("Texture vf4C input requires actual Texture Resource".into())}
        let file=self.read_source(handle)?;
        crate::resource_texture::TextureInput::from_selected(handle.clone(),file)
    }
    pub fn source_pixels_for_renderer(&self,source:&crate::resource_texture::SourceTexture)->Result<Option<crate::resource_texture::SourcePixels>>{
        let manager=self.manager.lock().map_err(|_|"Resource manager lock poisoned")?;
        let key=manager.texture_objects.key(source)?;manager.texture_objects.pixels_for_renderer(key)
    }
    pub(crate) fn retain_source_texture(&self,source:&crate::resource_texture::SourceTexture)->Result<()>{self.with_texture_objects(|a|{let key=a.key(source)?;a.retain_source(key)})}
    pub(crate) fn release_source_texture(&self,source:&crate::resource_texture::SourceTexture)->Result<()>{self.with_texture_objects(|a|{let key=a.key(source)?;a.release_source(key)})}
    pub fn source_reference_count(&self,source:&crate::resource_texture::SourceTexture)->Result<u32>{self.with_texture_objects(|a|{let key=a.key(source)?;a.source_refs(key)})}
    pub fn source_mode_48(&self,source:&crate::resource_texture::SourceTexture)->Result<u32>{self.with_texture_objects(|a|{let key=a.key(source)?;a.source_mode(key)})}
    ///Actual Source vf40 pixel publication, before Resource source3C and load
    ///flags. Native encoded layout and decoded RGBA payload are independent.
    ///Errors retain the SAME decoded allocation in the returned failure.
    pub fn publish_decoded_texture_40(&self,source:&crate::resource_texture::SourceTexture,decoded:crate::resource_texture::DecodedTexture,layout:&crate::texture_pixel_layout::NativePixelLayout)->std::result::Result<(),crate::resource_texture::DecodedPublicationFailure>{
        let mut decoded=Some(decoded);
        let result=self.with_texture_objects(|a|{
            let key=a.key(source)?;let mode=a.source_mode(key)?;
            decoded.as_ref().expect("same decoded owner").require_native_selection(layout,mode)?;
            let(format,widths,heights,offsets,stride)=layout.native_fields();
            let fields=crate::resource_texture_owner::PixelLayout{format,widths:widths.to_vec(),heights:heights.to_vec(),offsets:offsets.to_vec(),stride};
            match a.store_decoded(key,decoded.take().expect("same decoded owner"),fields){
                Ok(())=>Ok(()),Err(failure)=>{decoded=Some(failure.decoded);Err(failure.error)}
            }
        });
        match result{Ok(())=>Ok(()),Err(error)=>Err(crate::resource_texture::DecodedPublicationFailure{decoded:decoded.expect("failed publication retains decoded allocation"),error})}
    }
    ///Retains an actual modern reader's failed decoded prefix on this SAME
    ///native Source owner; a temporary client service must not discard it.
    pub fn retain_failed_decoded_40(&self,source:&crate::resource_texture::SourceTexture,decoded:crate::resource_texture::DecodedTexture)->std::result::Result<(),crate::resource_texture::DecodedPublicationFailure>{
        let mut decoded=Some(decoded);
        let result=self.with_texture_objects(|a|{
            let key=a.key(source)?;
            match a.retain_failed_decoded(key,decoded.take().expect("same decoded owner")){
                Ok(())=>Ok(()),Err(failure)=>{decoded=Some(failure.decoded);Err(failure.error)}
            }
        });
        match result{Ok(())=>Ok(()),Err(error)=>Err(crate::resource_texture::DecodedPublicationFailure{decoded:decoded.expect("failed retention returns decoded allocation"),error})}
    }
    pub fn memory_words_0c_10(&self,handle:&crate::resource_objects::Handle)->Result<[u32;2]>{self.with_entry(handle,|e|Ok([e.memory0c,e.memory10]))}
    ///Retains the actual GPU allocation at this SAME source2C only after its
    ///private source/pixel/device correlation validates. No ready flags change.
    pub fn attach_source_renderer_2c(&mut self,pixels:&crate::resource_texture::SourcePixels,renderer:Arc<dyn crate::resource_texture::RendererTexture>)->Result<()>{
        self.manager.lock().map_err(|_|"Resource manager lock poisoned")?.texture_objects.attach_renderer(pixels,renderer)
    }
}

struct TextureSettingsEntry<'a,T>{entry:&'a mut crate::resource_objects::Entry,policy:&'a mut T}
impl<T:crate::resource_objects::TexturePolicy> crate::resource_texture_settings::EntryWorld for TextureSettingsEntry<'_,T>{
    type Texture=();
    fn flags_08(&mut self,_:&())->Result<u32>{if self.entry.texture.is_none(){return Err("Texture options require actual Texture".into())}Ok(self.entry.flags8)}
    fn requests_04(&mut self,_:&())->Result<u32>{Ok(self.entry.requests4)}
    fn store_flags_08(&mut self,_:&(),v:u32)->Result<()>{self.entry.flags8=v;Ok(())}
    fn warning_404ed0(&mut self,_:&(),reading:bool)->Result<()>{self.policy.texture_option_warning_404ed0(self.entry.handle.logical_ids(),reading)}
    //with_entry holds the SAME manager mutex over this entire leaf, including
    //its prefix stores. Native mutex ABI/OS scheduling is not reproduced; the
    //admitted source-preparation loan is exclusive and has no mutable reentry.
    fn lock_857e10(&mut self,_:&())->Result<()>{Ok(())}
    fn loaded_reading_child(&mut self,_:&(),enabled:bool)->Result<()>{Err(if enabled{"5A9330 loaded Texture requires actual5A8B40→5A4F60→5A6A10 reading-source reload"}else{"5A9330 loaded Texture requires actual SourceTexture vf48 disable-reading"}.into())}
    fn unlock_857de0(&mut self,_:&())->Result<()>{Ok(())}
}

///Actual backend child services; default methods refuse the reached service.
///The internal wrapper ALWAYS uses the SAME manager's flags and post-load
///stores. A decoder callback cannot supply another mutable flag snapshot.
pub trait Backend:crate::resource_task::Services{
    fn load_resource_4c_in(&mut self,_resources:&mut Resources<'_>,_resource:&crate::resource_objects::Handle)->Result<()>{Err("actual Resource vf4C source/decoder backend is required".into())}
    ///Historical method name; actual5A5410 is shared by NiAVObj Model and
    ///NiSequence. The concrete backend must dispatch on the SAME Handle kind.
    fn model_after_base_3c_in(&mut self,_resources:&mut Resources<'_>,_resource:&crate::resource_objects::Handle)->Result<()>{Err("5A5410 actual Ni cache entry after flagsOR6 is required".into())}
    fn double_request_warning(&mut self,_resource:&crate::resource_objects::Handle,_count:i32)->Result<()>{Err("5AA3F0 actual double-request diagnostic404FC0 is required".into())}
    ///5AC3D0 reads the SAME current indexed UVar54 ResourceCaching cell
    ///([B39940]+D8) only when flags+8 bit3 is clear.
    fn resource_caching_54(&mut self,_resource:&crate::resource_objects::Handle)->Result<u8>{Err("5AC3D0 actual current indexed UVar54 ResourceCaching is required".into())}
    ///5AC3D0/5ABFF0 manager+3C guard around an unload (+20 uses enter_request).
    fn enter_unload_3c(&mut self,_resource:&crate::resource_objects::Handle)->Result<()>{Err("5AC3D0/5ABFF0 actual ResourceManager+3C guard is required".into())}
    fn leave_unload_3c(&mut self,_resource:&crate::resource_objects::Handle)->Result<()>{Err("5AC3D0/5ABFF0 actual ResourceManager+3C guard release is required".into())}
    ///404F60/404F90 ResourceManager release/unload diagnostics, in call order.
    fn release_log(&mut self,_resource:&crate::resource_objects::Handle,_log:&release::ReleaseLog)->Result<()>{Err("ResourceManager release/unload 404F60/404F90 diagnostic sink is required".into())}
    ///5A6560 calls 73D4B0(+40) for a Model whose flags+8 bit17 is set.
    fn detach_model_root_73d4b0(&mut self,_resource:&crate::resource_objects::Handle)->Result<()>{Err("5A6560 bit17 actual 73D4B0 NiAVObject detach is required".into())}
}
struct ResourceCallbacks<'a,'r,B>{resources:&'a mut Resources<'r>,backend:&'a mut B}
impl<B:Backend+crate::resource_task::DispatchServices>crate::resource_task::DispatchServices for ResourceCallbacks<'_,'_,B>{
    fn enter_dispatch(&mut self,application:&crate::resource_task::ApplicationKey,thread:&crate::resource_task::ThreadHandle)->Result<()>{self.backend.enter_dispatch(application,thread)}
    fn dispatch_diagnostic(&mut self,kind:crate::resource_task::DispatchKind<'_>)->Result<()>{self.backend.dispatch_diagnostic(kind)}
    fn leave_dispatch(&mut self)->Result<()>{self.backend.leave_dispatch()}
}
impl<B:Backend>crate::resource_task::Services for ResourceCallbacks<'_,'_,B>{
    fn current_thread(&mut self)->Result<crate::resource_task::ThreadHandle>{self.backend.current_thread()}
    fn enter_load(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.enter_load(r)}
    fn load_resource_4c(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.load_resource_4c_in(self.resources,r)}
    fn after_resource_3c(&mut self,r:&crate::resource_objects::Handle)->Result<()>{
        self.resources.with_entry(r,|v|{let old=v.flags8;v.flags8=old|4;v.flags8=old|6;Ok(())})?;
        if matches!(r.kind(),crate::resource_objects::Kind::Model|crate::resource_objects::Kind::Sequence){
            let flags=self.resources.flags_08(r)?;
            if flags&0x100==0{self.backend.model_after_base_3c_in(self.resources,r)?;}
        }
        Ok(())
    }
    fn resource_flags8(&mut self,r:&crate::resource_objects::Handle)->Result<u32>{self.resources.flags_08(r)}
    fn store_resource_flags8(&mut self,r:&crate::resource_objects::Handle,value:u32)->Result<()>{self.resources.with_entry(r,|v|{v.flags8=value;Ok(())})}
    fn loaded_diagnostic(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.loaded_diagnostic(r)}
    fn leave_load(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.leave_load(r)}
    fn enter_request(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.enter_request(r)}
    ///5B0270 under +20: the SAME manager's 59BFA0 cache removal.
    fn cached_resource_59bfa0(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.resources.cache_remove_59bfa0(r)}
    ///vf2C of all four admitted classes is the actual RET leaf 4B2830.
    fn cached_resource_virtual2c(&mut self,_r:&crate::resource_objects::Handle)->Result<()>{Ok(())}
    fn leave_request(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.leave_request(r)}
    fn request_diagnostic(&mut self,r:&crate::resource_objects::Handle,i:bool)->Result<()>{self.backend.request_diagnostic(r,i)}
    fn removing_from_queue_diagnostic(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.removing_from_queue_diagnostic(r)}
    fn block_timer_reset_59aa70(&mut self)->Result<()>{self.backend.block_timer_reset_59aa70()}
    fn block_timer_start_59ab60(&mut self)->Result<()>{self.backend.block_timer_start_59ab60()}
    fn timing_enabled_40eaa0(&mut self)->Result<bool>{self.backend.timing_enabled_40eaa0()}
    fn block_diagnostic(&mut self,r:&crate::resource_objects::Handle,d:crate::resource_task::BlockDiagnostic)->Result<()>{self.backend.block_diagnostic(r,d)}
    fn block_till_ready_5b1fd0(&mut self,t:&crate::resource_task::Task,q:&crate::resource_task::TaskQueue)->Result<()>{self.backend.block_till_ready_5b1fd0(t,q)}
    fn enter_manager_5abb30(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.enter_manager_5abb30(r)}
    fn leave_manager_5abb50(&mut self,r:&crate::resource_objects::Handle)->Result<()>{self.backend.leave_manager_5abb50(r)}
}
fn block_resource(task:&crate::resource_task::Task,queue:&crate::resource_task::TaskQueue,services:&mut impl crate::resource_task::Services)->Result<()>{
    if services.resource_flags8(task.resource())?&1==0{
        task.block_5b2180(queue,services)?;
        if services.resource_flags8(task.resource())?&1==0{return Err("5AAE10 actual Resource failed its load; ready flag remains clear".into());}
    }
    Ok(())
}
#[path="resource_cache.rs"]
mod cache;
#[path="resource_release.rs"]
pub mod release;
#[cfg(test)]
#[path="resource_texture_load_tests.rs"]
mod texture_load_tests;
#[cfg(test)]
#[path="resource_depot_missing_tests.rs"]
mod missing_tests;

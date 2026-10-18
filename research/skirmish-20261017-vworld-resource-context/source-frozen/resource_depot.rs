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
}
struct Directory {
    id: DepotId,
    path: PathBuf,
    writable: i32,
    priority: usize,
}
struct Manager {
    films: BTreeMap<String, Resource>,
    work: Option<Directory>,
    film_class: Option<crate::resource_cache_metadata::FilmClass>,
    warnings: Vec<String>,
    texture_subfolder_80: crate::vworld_resource_context::State,
}
impl Application {
    ///Adopt the actual already-verified source backend once. Native5B0400's
    ///name hashes start empty; this projection is not full ResourceManager
    ///construction, work-depot setup or an invented successful IO worker.
    pub fn from_sources(sources: V14Sources) -> Self {
        Self {
            origin: Arc::new(()),
            sources,
            maps: resource_map_cache::State::constructed(),
            manager: Mutex::new(Manager {
                films: BTreeMap::new(),
                work: None,
                film_class: None,
                warnings: Vec::new(),
                texture_subfolder_80: crate::vworld_resource_context::State::constructed(),
            }),
            process: None,
        }
    }
    pub fn sources_mut(&mut self) -> &mut V14Sources {
        &mut self.sources
    }
    ///43F0F1->5AE0A0 over this SAME manager. All currently admitted cache
    ///entries are typed MapResource/FilmResource allocations, including map
    ///Pending/Failed prefixes. Their class chains exclude NiAVObjResource,
    ///so5ACA50 skips each before reading flags/counts or unloading anything.
    ///A modern GPU cache may not be silently attached to this closed namespace.
    pub fn set_vworld_resource_context(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &crate::definitions::Bank,
        biome: &crate::vworld_resource_context::Biome,
    ) -> Result<crate::vworld_resource_context::Update> {
        let input = crate::vworld_resource_context::Input::bind(catalog, bank, biome)?;
        let mut manager = self.manager.lock().map_err(|_| "resource manager lock poisoned")?;
        let Manager { films, texture_subfolder_80, .. } = &mut *manager;
        texture_subfolder_80.assign_and_invalidate(&input.units, || {
            //These private owners have no generic insertion API. Native
            //Map B3D7DC->Resource B38E24 and Film B3D7F0->Map chains cannot
            //reach the unrelated NiAVObj descriptor B3D990. Bucket order
            //has no observable effect when EVERY entry takes that skip.
            let maps = self.maps.keys().count();
            let films = films.values().count();
            Ok((maps, films))
        })
    }
    ///A copied diagnostic value, never another mutable manager authority.
    pub fn vworld_texture_subfolder(&self) -> Result<Vec<u16>> {
        Ok(self.manager.lock().map_err(|_| "resource manager lock poisoned")?
            .texture_subfolder_80.current().to_vec())
    }
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

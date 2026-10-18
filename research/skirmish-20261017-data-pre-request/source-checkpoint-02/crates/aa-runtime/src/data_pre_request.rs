//! One application's canonical Data+20 request counters (4BBE70/4BB040/4BB050).
//! These are separate from Resource+reference counts and modern view leases.
//! This owner alone does NOT complete Object49B9F0 or Actor45DD80 child requests.
use crate::{catalog::Catalog, definitions::Bank};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::Precision;
use aa_rules::resources::{Provenance, SourceIdentity};
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};
#[path = "data_aspect.rs"]
pub mod aspect;
#[path = "data_pre_request_path.rs"]
pub mod path;
type Result<T> = std::result::Result<T, String>;

///Only4BBE70 constructs this scalar; request/release use native wrapping words.
#[derive(Debug)]
struct Counter(u32);
impl Counter {
    fn constructed() -> Self {
        Self(0)
    }
    fn request(&mut self, delta: u32, _mode: u32) {
        self.0 = self.0.wrapping_add(delta)
    }
    fn release(&mut self, delta: u32) {
        self.0 = self.0.wrapping_sub(delta)
    }
    fn requested(&self) -> bool {
        (self.0 as i32) > 0
    }
}
fn vfx_request(
    counter: &mut Counter,
    kind: u32,
    delta: u32,
    mode: u32,
    resource: impl FnOnce() -> Result<()>,
) -> Result<()> {
    if vfx_request_prefix(counter, kind, delta, mode)? {
        resource()
    } else {
        Ok(())
    }
}
fn vfx_request_prefix(counter: &mut Counter, kind: u32, delta: u32, mode: u32) -> Result<bool> {
    counter.request(delta, mode);
    match kind {
        5 | 6 | 7 | 8 | 9 | 12 | 13 | 15 | 16 | 22 | 23 => Ok(false),
        10 | 11 | 14 | 17 => Ok(true),
        _ => Err("VFX Data vf14 constructor dispatch is unbound".into()),
    }
}
#[derive(Debug)]
struct Entry {
    canonical: String,
    source: Provenance,
    counter: Counter,
    kind: u32,
    resource40: Resource40,
    aspects: Option<aspect::Object>,
}
enum Resource40 {
    Unbound,
    Stored {
        input: crate::view_vfx_source::Resource40Input,
        handle: Option<crate::resource_objects::Handle>,
        stage: Resource40Stage,
    },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Resource40Stage {
    Ready,
    TextureWarning,
    ModelFlags,
    ModelRead17,
}
struct RequiredModelPolicy;
impl crate::resource_objects::ModelPolicy for RequiredModelPolicy {
    fn use_hardware_model_73a560(&mut self) -> Result<bool> {
        Err("5A8520 requires actual current73A560 renderer policy".into())
    }
}
impl std::fmt::Debug for Resource40 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unbound => f.write_str("Unbound"),
            Self::Stored { handle, stage, .. } => f
                .debug_struct("StoredResource40")
                .field("present", &handle.is_some())
                .field("stage", stage)
                .finish(),
        }
    }
}
///An immutable identity capability for this SAME native Data allocation domain.
///No raw/token/default constructor is exposed, and a foreign owner is rejected.
#[derive(Clone, Debug)]
pub struct Data {
    owner: Weak<()>,
    token: u64,
}
impl Data {
    pub fn require_live(&self) -> Result<()> {
        self.owner
            .upgrade()
            .map(|_| ())
            .ok_or_else(|| "Data allocation owner was destroyed".into())
    }
    pub fn same_allocation(&self, other: &Self) -> bool {
        self.token == other.token
            && Weak::ptr_eq(&self.owner, &other.owner)
            && self.owner.upgrade().is_some()
    }
}
///Retain once before the application's first ported Data request; all future
///pre-request and release calls must reuse it. Recreating it per view is invalid.
///Immutable source receipts may be copied; request words have only this owner.
#[derive(Debug)]
pub struct State {
    owner: Arc<()>,
    entries: BTreeMap<u64, Entry>,
    names: BTreeMap<String, u64>,
    sources: Option<Vec<SourceIdentity>>,
    source_precision: Option<Precision>,
    ///B3A744 is a process assertion latch, not an Aspect ctor field. This
    ///owner covers one fresh interactive application interval (PE zero); it
    ///persists through all Data allocations/views and is never reset per map.
    aspect_assertion_suppressed: bool,
}
impl State {
    ///The directory initially has no bound Data instances. Each source-proven
    ///allocation enters it with4BBE70's actual zero+20, never a lease-derived count.
    pub fn from_native_constructor() -> Self {
        Self {
            owner: Arc::new(()),
            entries: BTreeMap::new(),
            names: BTreeMap::new(),
            sources: None,
            source_precision: None,
            aspect_assertion_suppressed: false,
        }
    }
    ///Bind one actual completed ObjectData/ActorData identity from the SAME
    ///source Bank. This does not execute any resource request or claim a model
    ///has loaded. Shared source reload replacement requires a separate owner.
    pub fn bind_object(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Data> {
        catalog.require_source_owner()?;
        let source = crate::live_object_init::Source::bind_in_bank(catalog, bank, definition)
            .map_err(|e| e.to_string())?;
        if !matches!(definition.native_class, 2 | 3 | 4 | 6 | 7 | 9) {
            return Err("Data request requires proven ObjectData/ActorData class".into());
        }
        let state = bank.combat_source()?;
        state.completed_means()?;
        if state.source_identities() != catalog.actors.sources.as_slice() {
            return Err("Data request source Bank universe differs".into());
        }
        let precision = source.source_precision();
        if self
            .sources
            .as_ref()
            .is_some_and(|s| s.as_slice() != state.source_identities())
            || self.source_precision.is_some_and(|p| p != precision)
        {
            return Err("Data request owner cannot reset or rebind its source universe/PC".into());
        }
        let token = bank.token("ActorDefinition", &definition.proof.id);
        if let Some(token) = token {
            let identity = bank.entry(token)?;
            if identity.key.domain != "actordefinition"
                || !identity.key.name.eq_ignore_ascii_case(&definition.proof.id)
            {
                return Err("Data request token/canonical source identity differs".into());
            }
        } else if !matches!(definition.proof.category.native_kind, 1 | 20 | 21) {
            return Err("Data request actual completed Actor source token absent".into());
        }
        //The CV source manager owns only native-kind2. Ordinary ObjectData has
        //a real Bank identity too, but cannot be read as a CV Actor definition.
        if definition.proof.category.native_kind == 2
            && (!state
                .definition_id(token.ok_or("Data Actor source token absent")?)?
                .eq_ignore_ascii_case(&definition.proof.id)
                || state.definition_source(token.ok_or("Data Actor source token absent")?)?
                    != &definition.proof.source)
        {
            return Err("Data request ActorData source-manager identity differs".into());
        }
        self.bind_canonical(
            &definition.proof.id,
            &definition.proof.source,
            definition.proof.category.native_kind,
            state.source_identities(),
            precision,
        )
    }
    ///Same canonical native Data directory for4BDB70's actual immutable VFX
    ///identity. No RuntimeDefinition or mutable/temporary Bank is fabricated.
    pub fn bind_vfx(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        effect: &crate::view_vfx_source::Effect,
    ) -> Result<Data> {
        effect.validate(catalog, bank)?;
        let state = bank.combat_source()?;
        let source = effect
            .sources()
            .first()
            .ok_or("VFX Data canonical source absent")?;
        self.bind_canonical(
            effect.id(),
            source,
            effect.kind(catalog, bank)?,
            state.source_identities(),
            state.source_precision(),
        )
    }
    ///Actual VFX vf14 dispatch:494380 tail-calls4BB040;494480 first calls
    ///the same base then requests its current optional+40 resource. The
    ///immutable Effect's constructor/vtable source proves this dispatch.
    pub fn pre_request_vfx(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        effect: &crate::view_vfx_source::Effect,
        delta: u32,
        mode: u32,
        resources: &mut impl VfxResources,
    ) -> Result<()> {
        let data = self.bind_vfx(catalog, bank, effect)?;
        let entry = self.entry_mut(&data)?;
        vfx_request(&mut entry.counter, entry.kind, delta, mode, || {
            resources.request_resource_40(effect, delta, mode)
        })
    }
    ///Same native vf14 through the concrete current ResourceManager. Source
    ///Resource40 MUST have been produced during the real pre-World type-load
    ///interval. This call never creates/reselects it after biome context moves.
    ///Data20's write remains before the current pointer read/Resource callback.
    pub fn pre_request_vfx_with_resources(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        effect: &crate::view_vfx_source::Effect,
        delta: u32,
        mode: u32,
        resources: &mut crate::resource_depot::Resources<'_>,
        backend: &mut impl crate::resource_depot::Backend,
    ) -> Result<()> {
        let data = self.bind_vfx(catalog, bank, effect)?;
        let resource = {
            let entry = self.entry_mut(&data)?;
            vfx_request_prefix(&mut entry.counter, entry.kind, delta, mode)?
        };
        if resource && let Some(handle) = self.vfx_resource40(catalog, bank, effect)? {
            resources.request_10(&handle, delta as i32, mode, backend)?;
        }
        Ok(())
    }
    ///Bind the selected native type-LoadData Resource40 field in this SAME
    ///Data allocation. This source operation is distinct from vf14 request:
    ///it must not be repeated per request or counted as whole LoadData.
    ///495570 calls actual Texture5A77E0(flags0), then stores40 before warning.
    ///Model495F10's predicate is required only if its native selector reaches
    ///it. Use the policy entry below with the actual retained renderer owner.
    pub fn prepare_vfx_resource40(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        effect: &crate::view_vfx_source::Effect,
        resources: &mut crate::resource_depot::Resources<'_>,
    ) -> Result<Option<crate::resource_objects::Handle>> {
        self.prepare_vfx_resource40_with_policy(
            catalog,
            bank,
            effect,
            resources,
            &mut RequiredModelPolicy,
        )
    }
    pub fn prepare_vfx_resource40_with_policy(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        effect: &crate::view_vfx_source::Effect,
        resources: &mut crate::resource_depot::Resources<'_>,
        policy: &mut impl crate::resource_objects::ModelPolicy,
    ) -> Result<Option<crate::resource_objects::Handle>> {
        let data = self.bind_vfx(catalog, bank, effect)?;
        if matches!(self.entry(&data)?.resource40, Resource40::Unbound) {
            let input = effect.resource_40_input(catalog, bank)?;
            let handle = match input.kind() {
                crate::resource_objects::Kind::Texture => {
                    resources.texture_5a77e0(catalog, bank, input.ids(), 0)?
                }
                crate::resource_objects::Kind::Model => {
                    Some(resources.model_5a8520(catalog, bank, input.ids(), policy)?)
                }
                crate::resource_objects::Kind::Sequence => {
                    return Err("VFX Resource40 cannot contain a Sequence resource".into());
                }
            };
            let stage = if input.kind() == crate::resource_objects::Kind::Model {
                Resource40Stage::ModelFlags
            } else if handle.is_none() {
                Resource40Stage::TextureWarning
            } else {
                Resource40Stage::Ready
            };
            //Native pointer store precedes every Model flag/source child.
            self.entry_mut(&data)?.resource40 = Resource40::Stored {
                input,
                handle,
                stage,
            };
        }
        loop {
            let (input, handle, stage) = match &self.entry(&data)?.resource40 {
                Resource40::Stored {
                    input,
                    handle,
                    stage,
                } => (input.clone(), handle.clone(), *stage),
                Resource40::Unbound => return Err("VFX Resource40 source store disappeared".into()),
            };
            input.validate(catalog, bank)?;
            match stage {
                Resource40Stage::Ready=>{
                    if let Some(h)=&handle{resources.flags_08(h)?;}
                    return Ok(handle)
                }
                Resource40Stage::TextureWarning=>return Err("495570 null Texture40 requires actual404ED0 warning sink; stored field retained".into()),
                Resource40Stage::ModelFlags=>{
                    let h=handle.as_ref().ok_or("495F10 nonnull Model source factory domain violated")?;
                    let unique=input.model_floating_ui(catalog,bank)?;
                    resources.vfx_model_flags(h,unique)?;
                    let next=if self.entry(&data)?.kind==17 {Resource40Stage::ModelRead17}else{Resource40Stage::Ready};
                    if let Resource40::Stored{stage,..}=&mut self.entry_mut(&data)?.resource40 {*stage=next;}
                }
                Resource40Stage::ModelRead17=>{
                    let h=handle.as_ref().ok_or("49631E current Model40 is null")?;
                    resources.set_load_for_reading_5a55c0(h,true)?;
                    if let Resource40::Stored{stage,..}=&mut self.entry_mut(&data)?.resource40 {*stage=Resource40Stage::Ready;}
                }
            }
        }
    }
    ///Read only an actually produced40. Unbound source fields are never null.
    ///Returned Handle clones share the SAME immutable allocation identity.
    pub fn vfx_resource40(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        effect: &crate::view_vfx_source::Effect,
    ) -> Result<Option<crate::resource_objects::Handle>> {
        let data = self.bind_vfx(catalog, bank, effect)?;
        match &self.entry(&data)?.resource40 {
            Resource40::Unbound => Err("494480 current Resource40 source field is unbound".into()),
            Resource40::Stored {
                input,
                handle,
                stage,
            } => {
                input.validate(catalog, bank)?;
                if *stage != Resource40Stage::Ready {
                    return Err("Resource40 source continuation still requires its real child; pointer retained".into());
                }
                Ok(handle.clone())
            }
        }
    }
    fn bind_canonical(
        &mut self,
        id: &str,
        source: &Provenance,
        kind: u32,
        sources: &[SourceIdentity],
        precision: Precision,
    ) -> Result<Data> {
        if self
            .sources
            .as_ref()
            .is_some_and(|s| s.as_slice() != sources)
            || self.source_precision.is_some_and(|p| p != precision)
        {
            return Err("Data request owner cannot reset or rebind its source universe/PC".into());
        }
        let name = id.to_ascii_lowercase();
        let token = if let Some(token) = self.names.get(&name) {
            let entry = self
                .entries
                .get(token)
                .ok_or("Data canonical directory allocation absent")?;
            if !entry.canonical.eq_ignore_ascii_case(id)
                || &entry.source != source
                || entry.kind != kind
            {
                return Err("Data request existing native allocation source/kind differs".into());
            }
            *token
        } else {
            let token = u64::try_from(self.entries.len())
                .ok()
                .and_then(|v| v.checked_add(1))
                .ok_or("portable Data identity exhausted")?;
            self.entries.insert(
                token,
                Entry {
                    canonical: id.to_owned(),
                    source: source.clone(),
                    kind,
                    counter: Counter::constructed(),
                    resource40: Resource40::Unbound,
                    aspects: None,
                },
            );
            self.names.insert(name, token);
            token
        };
        if self.sources.is_none() {
            self.sources = Some(sources.to_vec());
            self.source_precision = Some(precision);
        }
        Ok(Data {
            owner: Arc::downgrade(&self.owner),
            token,
        })
    }
    fn entry(&self, data: &Data) -> Result<&Entry> {
        if !data
            .owner
            .upgrade()
            .is_some_and(|p| Arc::ptr_eq(&p, &self.owner))
        {
            return Err("foreign Data request owner".into());
        }
        self.entries
            .get(&data.token)
            .ok_or("Data request allocation absent".into())
    }
    fn entry_mut(&mut self, data: &Data) -> Result<&mut Entry> {
        self.entry(data)?;
        self.entries
            .get_mut(&data.token)
            .ok_or("Data request allocation absent".into())
    }
    ///4BB040. This store precedes every Object/Actor child request. Mode is
    ///passed to children but deliberately unread by the native base function.
    pub fn request_base(&mut self, data: &Data, delta: u32, mode: u32) -> Result<()> {
        self.entry_mut(data)?.counter.request(delta, mode);
        Ok(())
    }
    ///4BB050, same allocation and word as request_base; no saturation/clamp.
    pub fn release_base(&mut self, data: &Data, delta: u32) -> Result<()> {
        self.entry_mut(data)?.counter.release(delta);
        Ok(())
    }
    ///4BB060 signed-positive test, not nonzero and not modern GPU residency.
    pub fn requested_base(&self, data: &Data) -> Result<bool> {
        Ok(self.entry(data)?.counter.requested())
    }
    pub fn count_word(&self, data: &Data) -> Result<u32> {
        Ok(self.entry(data)?.counter.0)
    }
}
///The reached494480 optional-resource field and native resource vf10. A
///concrete source/resource owner must evaluate the null guard here; absence
///of an implementation never means an invented null resource.
pub trait VfxResources {
    fn request_resource_40(
        &mut self,
        effect: &crate::view_vfx_source::Effect,
        delta: u32,
        mode: u32,
    ) -> Result<()>;
}
pub struct RequiredVfxResources;
impl VfxResources for RequiredVfxResources {
    fn request_resource_40(
        &mut self,
        _: &crate::view_vfx_source::Effect,
        _: u32,
        _: u32,
    ) -> Result<()> {
        Err(
            "VFX494480 requires actual current Data+40 resource/source owner; base+20 retained"
                .into(),
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_data_request_constructor_wrapping_and_signed_requested_words() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-data-pre-request/native-data-request-base.json"
        ))
        .unwrap();
        assert_eq!(doc["rows"].as_array().unwrap().len(), 36);
        for row in doc["rows"].as_array().unwrap() {
            let mut counter = Counter::constructed();
            let mut output =
                vec![serde_json::json!({"word":counter.0,"requested":counter.requested()})];
            for operation in row["input"].as_array().unwrap() {
                let delta = operation[1].as_u64().unwrap() as u32;
                if operation[0] == "request" {
                    counter.request(delta, operation[2].as_u64().unwrap() as u32)
                } else {
                    counter.release(delta)
                }
                output.push(serde_json::json!({"word":counter.0,"requested":counter.requested()}));
            }
            assert_eq!(
                serde_json::json!(output),
                row["output"],
                "{} / {}",
                row["input"],
                row["precision"]
            );
        }
    }
    #[test]
    fn data_identity_refuses_foreign_or_destroyed_counter_owner() {
        let owner = State::from_native_constructor();
        let data = Data {
            owner: Arc::downgrade(&owner.owner),
            token: 1,
        };
        let mut foreign = State::from_native_constructor();
        assert_eq!(
            foreign.request_base(&data, 1, 0).unwrap_err(),
            "foreign Data request owner"
        );
        drop(owner);
        assert_eq!(
            foreign.release_base(&data, 1).unwrap_err(),
            "foreign Data request owner"
        );
        assert!(foreign.entries.is_empty());
    }
    #[test]
    fn native_vfx_request_dispatch_and_base_store_before_required_child() {
        let doc: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-data-pre-request/native-vfx-request.json"
        ))
        .unwrap();
        assert_eq!(doc["rows"].as_array().unwrap().len(), 675);
        for row in doc["rows"].as_array().unwrap() {
            let input = &row["input"];
            let mut counter = Counter(input["initial"].as_u64().unwrap() as u32);
            let delta = input["delta"].as_u64().unwrap() as u32;
            let mode = input["mode"].as_u64().unwrap() as u32;
            let mut calls = Vec::new();
            let result = vfx_request(
                &mut counter,
                input["kind"].as_u64().unwrap() as u32,
                delta,
                mode,
                || {
                    if input["resource"] != "null" {
                        calls.push([delta, mode]);
                    }
                    if input["resource"] == "required" {
                        Err("required".into())
                    } else {
                        Ok(())
                    }
                },
            );
            let trace: Vec<_> = calls
                .iter()
                .map(|args| serde_json::json!({"args":args,"word":counter.0}))
                .collect();
            assert_eq!(
                serde_json::json!({"word":counter.0,"trace":trace,"status":if result.is_ok(){"ok"}else{"required"}}),
                row["output"],
                "{} / {}",
                input,
                row["precision"]
            );
        }
    }
    #[test]
    fn canonical_data_directory_reuses_counter_and_rejects_source_replacement() {
        //Explicit private identity-domain lab; not an authored source loader.
        let source = Provenance {
            path: "lab.tgi".into(),
            sha256: "lab-only".into(),
            span: aa_rules::tgi::Span { start: 1, end: 2 },
        };
        let mut state = State::from_native_constructor();
        let data = state
            .bind_canonical("VFX", &source, 12, &[], Precision::Double53)
            .unwrap();
        state.request_base(&data, 7, 0).unwrap();
        let same = state
            .bind_canonical("vfx", &source, 12, &[], Precision::Double53)
            .unwrap();
        assert_eq!(state.count_word(&same).unwrap(), 7);
        assert_eq!(state.entries.len(), 1);
        let mut changed = source.clone();
        changed.span.end = 3;
        assert!(
            state
                .bind_canonical("VFX", &changed, 12, &[], Precision::Double53)
                .is_err()
        );
        assert!(
            state
                .bind_canonical("VFX", &source, 13, &[], Precision::Double53)
                .is_err()
        );
        assert!(
            state
                .bind_canonical("VFX", &source, 12, &[], Precision::Single24)
                .is_err()
        );
        assert_eq!(state.count_word(&data).unwrap(), 7);
    }
    #[test]
    #[ignore = "requires actual source-owned Catalog/Bank and native Resource field constructors"]
    fn actual_projected_vfx_resource40_retains_same_source_manager_allocation() {
        use crate::{
            combat_source_receipt::VerifiedStartupReceipt, load_view_variables::NativeOverrides,
        };
        use aa_rules::{
            actors::ActorCatalog,
            sources::{SourceLimits, V14Sources},
            tgi::Limits,
        };
        let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut files = V14Sources::open(
            game.join("Data.rwd"),
            game.join("reconstruction/native-game/runtime-expanded-v14"),
            SourceLimits {
                max_member_bytes: 64 * 1024 * 1024,
            },
        )
        .unwrap();
        let actors = ActorCatalog::load_v14(&mut files).unwrap();
        let mut catalog = Catalog::new(&actors);
        catalog.settings =
            Some(crate::settings::Settings::load_v14(&mut files, Limits::default()).unwrap());
        let receipt =
            VerifiedStartupReceipt::verify_embedded_reference(&catalog, &mut files).unwrap();
        crate::combat_source_completion::install(
            &catalog,
            &mut files,
            receipt.context(),
            receipt.csv(),
            receipt.globals(),
        )
        .unwrap();
        crate::combat_source_completion::compute(&catalog).unwrap();
        let aliases = crate::view_asset_aliases::State::from_fresh_sources(
            &catalog,
            &mut files,
            NativeOverrides::ControlledNone,
        )
        .unwrap();
        let bank = std::mem::take(&mut *catalog.definition_bank.borrow_mut());
        let mut texture = None;
        let mut text = None;
        for key in crate::view_init_assets::alias_keys().filter(|k| !k.starts_with("Texture")) {
            let alias = aliases.alias_410cb0(&catalog, &bank, key).unwrap();
            let effect = crate::view_vfx_source::find(&catalog, &bank, alias.units())
                .unwrap()
                .unwrap();
            match effect.kind(&catalog, &bank).unwrap() {
                11 if texture.is_none() => texture = Some(effect),
                12 if text.is_none() => text = Some(effect),
                _ => (),
            }
        }
        let texture = texture.expect("actual current projected Texture VFX alias");
        let text = text.expect("actual current text VFX alias");
        let input = texture.resource_40_input(&catalog, &bank).unwrap();
        assert_eq!(input.kind(), crate::resource_objects::Kind::Texture);
        let mut app = crate::resource_depot::Application::from_sources(files);
        app.with_data_pre_request_resources(|state,resources| {
            let data=state.bind_vfx(&catalog,&bank,&texture)?;
            assert!(state.vfx_resource40(&catalog,&bank,&texture).is_err());
            let handle=state.prepare_vfx_resource40(&catalog,&bank,&texture,resources)?.unwrap();
            let references=resources.references_14(&handle)?;
            assert_eq!(state.count_word(&data)?,0);
            assert_eq!(resources.requests_04(&handle)?,0);
            assert_eq!(resources.flags_08(&handle)?&1,0);
            let again=state.prepare_vfx_resource40(&catalog,&bank,&texture,resources)?.unwrap();
            assert!(handle.same_identity(&again));
            assert_eq!(resources.references_14(&handle)?,references);
            assert!(state.vfx_resource40(&catalog,&bank,&texture)?.unwrap().same_identity(&handle));
            //The selected source pointer is bound, but missing request10 is
            //still a real error after native Data20's committed increment.
            assert!(state.pre_request_vfx(&catalog,&bank,&texture,1,0,&mut RequiredVfxResources).is_err());
            assert_eq!(state.count_word(&data)?,1);
            state.pre_request_vfx(&catalog,&bank,&text,1,0,&mut RequiredVfxResources)?;
            let text_data=state.bind_vfx(&catalog,&bank,&text)?;
            assert_eq!(state.count_word(&text_data)?,1);
            assert!(input.validate(&catalog,&crate::definitions::Bank::default()).is_err());
            println!("actual VFX {} Resource40 same allocation/source {:?}; no decoder or full VFX Loader completion",texture.id(),handle.source_file().logical_path());
            Ok(())
        }).unwrap();
    }
}

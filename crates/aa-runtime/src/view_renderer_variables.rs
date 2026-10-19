//! Sole current AVar25 RendererDefaultOrientation Point3 owner. Native401BD0
//! registers401880's [0,-1,0], kind5/flags0;4B3C10 case8 dispatches4B32B0,
//! whose flags0 Point3 setter4B2B60 copies the three words unchanged.
use crate::{catalog::Catalog,definitions::Bank,combat_value_source_context::{self,SourceFpContext},load_view_variables::NativeOverrides};
use aa_rules::{resources::{Provenance,SourceIdentity},sources::V14Sources,tgi::{Document,Item,Limits},native::{index::native_enumerate,scalar_literal}};
type Result<T>=std::result::Result<T,String>;
pub const NAME:&str="RendererDefaultOrientation";
const CONSTRUCTOR:[u32;3]=[0,0xbf800000,0];
///No Default/Clone/raw-value constructor. This is the current named cell, not
///a copied per-object orientation or a second whole AVar manager.
pub struct State {
    current:[u32;3],sources:std::sync::Arc<Vec<SourceIdentity>>,source_context:SourceFpContext,
    source_files:Vec<(String,String)>,assignments:Vec<Provenance>,raw_assignments:Vec<String>,source_loaded:bool,
}
pub struct Failure {pub state:Option<State>,pub error:String}
impl std::fmt::Debug for Failure{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("RendererOrientationFailure").field("owner_retained",&self.state.is_some()).field("error",&self.error).finish()}}
impl State {
    pub fn from_fresh_sources(catalog:&Catalog<'_>,bank:&Bank,files:&mut V14Sources,overrides:NativeOverrides)->Result<Self>{
        Self::from_fresh_sources_retained(catalog,bank,files,overrides).map_err(|failure|failure.error)
    }
    ///Failure owners retain the same reached current cell/source assignment
    ///prefix. A failed preflight has no source-certified allocation to retain.
    pub fn from_fresh_sources_retained(catalog:&Catalog<'_>,bank:&Bank,files:&mut V14Sources,overrides:NativeOverrides)->std::result::Result<Self,Failure>{
        let allocation=(||->Result<Self>{
            catalog.require_source_owner()?;
            if !matches!(overrides,NativeOverrides::ControlledNone){return Err("RendererDefaultOrientation current profile/console AVar owner is unbound".into());}
            let owner=bank.combat_source()?;owner.completed_means()?;
            if owner.source_identities()!=catalog.actors.sources.as_slice(){return Err("RendererDefaultOrientation source Bank differs".into());}
            Ok(Self{current:CONSTRUCTOR,sources:crate::catalog_lookups::catalog_universe(catalog),source_context:owner.context().clone(),source_files:Vec::new(),assignments:Vec::new(),raw_assignments:Vec::new(),source_loaded:false})
        })();
        let mut state=allocation.map_err(|error|Failure{state:None,error})?;
        match state.load_sources(catalog,bank,files){Ok(())=>Ok(state),Err(error)=>Err(Failure{state:Some(state),error})}
    }
    fn load_sources(&mut self,catalog:&Catalog<'_>,bank:&Bank,files:&mut V14Sources)->Result<()> {
        let owner=bank.combat_source()?;
        let names=files.catalog().into_iter().map(|entry|entry.logical_path).collect::<Vec<_>>();
        let ordered=native_enumerate("AVars*.tgi",&names);
        if !ordered.iter().any(|p|p.eq_ignore_ascii_case("AVars.tgi")){return Err("RendererDefaultOrientation actual root AVars source is absent".into());}
        for path in ordered {
            let file=files.read(&path).map_err(|e|e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes)!=file.sha256{return Err("RendererDefaultOrientation AVar source hash differs".into());}
            self.source_files.push((file.logical_path.clone(),file.sha256.clone()));
            let doc=Document::parse_native(file.bytes,Limits::default()).map_err(|e|e.to_string())?;
            for item in &doc.items {
                let Item::Node(node)=item else{continue};
                if !doc.header_name(&node.header).is_some_and(|v|v.eq_ignore_ascii_case("Vars")){continue;}
                if node.header.tokens.len()!=1{return Err("RendererDefaultOrientation inherited Vars source lookup is unbound".into());}
                for item in &node.items {
                    let Item::Property(property)=item else{continue};
                    if !doc.property_name(property).is_some_and(|v|v.eq_ignore_ascii_case(NAME)){continue;}
                    let kind=if property.left.len()==2{doc.text(property.left[0].span)}else{None};
                    let raw=doc.property_value(property).ok_or("RendererDefaultOrientation Point3 assignment absent")?;
                    let source=Provenance{path:file.logical_path.clone(),sha256:file.sha256.clone(),span:property.span};
                    let rows=&catalog.settings.as_ref().ok_or("RendererDefaultOrientation Settings owner absent")?.app_variables;
                    let row=rows.get(&NAME.to_ascii_lowercase()).and_then(|rows|rows.get(self.assignments.len())).ok_or("RendererDefaultOrientation public declaration position absent")?;
                    if row.source!=source||row.raw.trim()!=raw||!row.field_type.as_deref().is_some_and(|v|v.eq_ignore_ascii_case("point3")){
                        return Err("RendererDefaultOrientation public/private source span/type/raw differs".into());
                    }
                    let value=assignment(kind,Some(raw),owner.source_precision())?;
                    self.current=value;
                    self.assignments.push(source);self.raw_assignments.push(raw.to_owned());
                }
            }
        }
        self.validate_owner(catalog,bank)?;self.source_loaded=true;Ok(())
    }
    pub fn validate(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<()> {
        if !self.source_loaded{return Err("RendererDefaultOrientation actual source assignment pass is incomplete".into());}
        self.validate_owner(catalog,bank)
    }
    fn validate_owner(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<()> {
        catalog.require_source_owner()?;
        let source=bank.combat_source()?;source.completed_means()?;
        if !crate::catalog_lookups::same_sources(catalog,&self.sources)||!crate::catalog_lookups::same_slices(source.source_identities(),&self.sources)
            ||source.source_precision()!=self.source_context.precision(){return Err("RendererDefaultOrientation destination source Bank/precision differs".into());}
        let rows=catalog.settings.as_ref().ok_or("RendererDefaultOrientation Settings owner absent")?.app_variables.get(&NAME.to_ascii_lowercase()).map(Vec::as_slice).unwrap_or(&[]);
        if rows.len()!=self.assignments.len()||self.assignments.len()!=self.raw_assignments.len()
            ||rows.iter().zip(&self.assignments).zip(&self.raw_assignments).any(|((row,provenance),raw)|row.source!=*provenance||row.raw.trim()!=raw
                ||!row.field_type.as_deref().is_some_and(|v|v.eq_ignore_ascii_case("point3"))){
            return Err("RendererDefaultOrientation current Settings declaration provenance/type/raw differs".into());
        }
        let numeric=combat_value_source_context::bind_numeric(catalog,&self.source_context)?;
        crate::combat_source_receipt::require_reference_numeric(&numeric)
    }
    ///Actual current Point3+4/+8/+C; constructor values are only used if the
    ///complete real native-ordered root source set naturally has no assignment.
    pub fn current(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<[u32;3]>{self.validate(catalog,bank)?;Ok(self.current)}
    pub fn source_files(&self)->&[(String,String)]{&self.source_files}
    pub fn assignments(&self)->&[Provenance]{&self.assignments}
}
fn assignment(kind:Option<&str>,raw:Option<&str>,pc:aa_numeric::Precision)->Result<[u32;3]>{
    if !kind.is_some_and(|v|v.eq_ignore_ascii_case("point3")){return Err("RendererDefaultOrientation requires the actual declared Point3 source type".into());}
    let raw=raw.ok_or("RendererDefaultOrientation Point3 assignment absent")?;
    let parts=raw.split(',').collect::<Vec<_>>();
    if parts.len()!=3{return Err("RendererDefaultOrientation Point3 requires exactly three source scalars".into());}
    let mut value=[0;3];for (i,text)in parts.into_iter().enumerate(){value[i]=scalar_literal(text,pc).map_err(|e|e.to_string())?;}Ok(value)
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn actual_native_constructor_registration_and_point3_assignment_all_pc(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-renderer-orientation.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap(){
            if let Some(reg)=row.get("registration"){
                assert_eq!(reg["index"],25);assert_eq!(reg["kind"],5);assert_eq!(reg["flags"],0);
                assert_eq!(String::from_utf16(&reg["name"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u16).collect::<Vec<_>>()).unwrap(),NAME);
                assert_eq!(serde_json::json!(CONSTRUCTOR),reg["value"]);
            }else{assert_eq!(serde_json::json!(CONSTRUCTOR),row["before"]);assert_eq!(row["after"],row["input"]);assert_eq!(row["kind"],5);assert_eq!(row["flags"],0);}
        }
    }
    #[test]fn conservative_typed_source_literals_refuse_unknown_expressions(){
        for pc in [aa_numeric::Precision::Single24,aa_numeric::Precision::Double53,aa_numeric::Precision::Extended64]{
            assert_eq!(assignment(Some("point3"),Some("0,-1,0"),pc).unwrap(),CONSTRUCTOR);
            assert_eq!(assignment(Some("POINT3"),Some("-0, 1, 0.5"),pc).unwrap(),[0x80000000,0x3f800000,0x3f000000]);
            for (kind,raw)in [(None,Some("0,-1,0")),(Some("float"),Some("0,-1,0")),(Some("point3"),None),(Some("point3"),Some("0,1")),(Some("point3"),Some("0,1,2,3")),(Some("point3"),Some("0,$Other,0")),(Some("point3"),Some("0,1+1,0"))]{assert!(assignment(kind,raw,pc).is_err());}
        }
    }
    #[test]#[ignore="requires actual original/v14 sources; no native engine or GPU execution"]
    fn actual_source_same_completed_bank_and_retained_scope_refusal(){
        use aa_rules::sources::{V14Sources,SourceLimits};
        let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
        let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
        let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
        let mut catalog=Catalog::new(&actors);catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
        let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
        crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
        crate::combat_source_completion::compute(&catalog).unwrap();
        let bank=catalog.definition_bank.borrow();
        let state=State::from_fresh_sources_retained(&catalog,&bank,&mut files,NativeOverrides::ControlledNone).unwrap();
        assert_eq!(state.current(&catalog,&bank).unwrap(),CONSTRUCTOR);assert_eq!(state.assignments().len(),1);assert!(!state.source_files().is_empty());
        let source=&state.assignments()[0];let file=files.read(&source.path).unwrap();assert_eq!(file.sha256,source.sha256);
        let doc=Document::parse_native(file.bytes,Limits::default()).unwrap();let raw=doc.text(source.span).unwrap();assert!(raw.contains(NAME)&&raw.contains("0,-1,0"));
        let failure=State::from_fresh_sources_retained(&catalog,&bank,&mut files,NativeOverrides::Unbound).err().unwrap();assert!(failure.state.is_none());
        let raw=catalog.settings.as_ref().unwrap().app_variables[&NAME.to_ascii_lowercase()][0].raw.clone();
        catalog.settings.as_mut().unwrap().app_variables.get_mut(&NAME.to_ascii_lowercase()).unwrap()[0].raw="0,1,0".into();
        assert!(state.current(&catalog,&bank).unwrap_err().contains("declaration provenance/type/raw"));
        catalog.settings.as_mut().unwrap().app_variables.get_mut(&NAME.to_ascii_lowercase()).unwrap()[0].raw=raw;
        assert_eq!(state.current(&catalog,&bank).unwrap(),CONSTRUCTOR);
    }
}

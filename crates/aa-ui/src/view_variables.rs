//! Native43F156..43F1B2 reset over the213 registered UVars. Only eight
//! reset Bool cells are reached; RendererPreRequestAllVThings also has one
//! canonical Bool cell for native45B530. RendererUseHWSkinningShader is owned
//! for73A560 outside the reset loop; DevMode is the same current UVar62 byte
//! read by40EAA0. The remaining entries are lookup
//! metadata, not duplicated current values. CameraInformation is stored here
//! once and borrowed by the existing UVars/HUD/options accessors.
//! ResourceNoNiTextures is a separate source-created named Bool in this SAME
//! owner; it is absent before4B3C10 and has no fabricated indexed registration.
use crate::fs::DataRoot;
use aa_rules::{native::index::native_enumerate, tgi::{Document,Item,Limits}};

#[path="view_variable_registration.rs"]
mod registration;
#[cfg(test)]
#[path="view_variables_tests.rs"]
mod tests;

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub struct Registration {
    pub index:u16,
    pub name:&'static str,
    pub native_kind:u32,
    pub flags:u32,
}
pub fn registrations()->&'static [Registration] {&registration::REGISTRATIONS}
pub const MUTABLE_INDICES:[u16;8]=[22,24,25,27,29,44,100,101];
pub const OWNED_BOOL_INDICES:[u16;14]=[22,24,25,27,29,44,100,101,108,113,62,61,52,54];
///Source-created4B3C10 Bool, looked up by NAME at5A8D5F. It has no index
///in the213 constructor registrations and no constructor default allocation.
pub(crate) const RESOURCE_NO_NI_TEXTURES:&str="ResourceNoNiTextures";
///5A99FA looks this source-created Bool up by name(kind1), not an index.
pub(crate) const RESOURCE_NO_NI_OBJECTS:&str="ResourceNoNiObjects";
///423A31 creates this named Bool with flags0/current+default1, then actual
///UVars sources update its current byte (original true, AA false).
pub(crate) const DATA_MISSING_RESOURCE_WARNING:&str="DataMissingResourceWarning";
///Named41F9D0 suffix registrations via4B35F0, outside the213 indexed
///directory. This array describes local storage only, never native indices.
pub(crate) const TEXTURE_QUALITY_NAMES:[&str;2]=["RendererForceLowQualityTextures","RendererUseHighQualityTextures"];
fn quality_slot(name:&str)->Option<usize>{TEXTURE_QUALITY_NAMES.iter().position(|n|n.eq_ignore_ascii_case(name))}
///5A8BF1..5A8C2E reads Force first; only zero reaches the High lookup/read.
pub(crate) fn texture_quality_source_mode(mut read:impl FnMut(&'static str)->Result<u8,String>)->Result<bool,String>{
    if read(TEXTURE_QUALITY_NAMES[0])?!=0 {return Ok(true)}
    Ok(read(TEXTURE_QUALITY_NAMES[1])?==0)
}

/// Explicit startup scope. A profile/console producer cannot be inferred from
/// source defaults. Existing private UI profile writes use the same byte cells.
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub enum SourceOverrides {Unbound,ControlledNone}

#[derive(Clone,Debug,PartialEq,Eq)]
pub struct SourceFile {pub path:String,pub sha256:String}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct BoolCell {current:u8,reset_default:u8,source_snapshot:u8}
#[derive(Clone,Debug)]
pub(crate) struct State {
    cells:[BoolCell;14],
    resource_no_ni_textures:Option<BoolCell>,
    resource_no_ni_objects:Option<BoolCell>,
    texture_quality:[BoolCell;2],
    data_missing_resource_warning:BoolCell,
    source_files:Vec<SourceFile>,
    failure:Option<String>,
    source_bound:bool,
}
impl Default for State {fn default()->Self {Self::from_native_constructors()}}
impl State {
    ///4017D0 from the real registration capture: the eight reset targets
    ///start0/0; RendererPreRequestAllVThings and RendererUseHWSkinningShader
    ///start1/1. This is not by itself
    ///a source/profile completion certificate.
    pub(crate) fn from_native_constructors()->Self {
        let mut cells=[BoolCell {current:0,reset_default:0,source_snapshot:0};14];
        cells[8]=BoolCell {current:1,reset_default:1,source_snapshot:1};
        cells[9]=BoolCell {current:1,reset_default:1,source_snapshot:1};
        //41F9D0 UVar61: kind1, flags0, Bool current/default1.
        cells[11]=BoolCell {current:1,reset_default:1,source_snapshot:1};
        //41F9D0 UVar52 RendererRemoveNormals starts0/0; UVar54
        //ResourceCaching starts1/1. Both are Bool flags0.
        cells[13]=BoolCell {current:1,reset_default:1,source_snapshot:1};
        //Actual424393/4243D3 constructors: High current/default1 then
        //Force current/default0, both Bool flags16 and named append4B35F0.
        let texture_quality=[BoolCell {current:0,reset_default:0,source_snapshot:0},BoolCell {current:1,reset_default:1,source_snapshot:1}];
        Self {cells,resource_no_ni_textures:None,resource_no_ni_objects:None,texture_quality,
            data_missing_resource_warning:BoolCell {current:1,reset_default:1,source_snapshot:1},
            source_files:Vec::new(),failure:None,source_bound:false}
    }
    fn slot(name:&str)->Option<usize> {
        OWNED_BOOL_INDICES.iter().position(|&i|registration::REGISTRATIONS[i as usize].name.eq_ignore_ascii_case(name))
    }
    pub(crate) fn contains(name:&str)->bool {Self::slot(name).is_some() || name.eq_ignore_ascii_case(RESOURCE_NO_NI_TEXTURES) || name.eq_ignore_ascii_case(RESOURCE_NO_NI_OBJECTS) || quality_slot(name).is_some() || name.eq_ignore_ascii_case(DATA_MISSING_RESOURCE_WARNING)}
    pub(crate) fn raw(&self,name:&str)->Option<u8> {
        if name.eq_ignore_ascii_case(RESOURCE_NO_NI_TEXTURES){return self.resource_no_ni_textures.as_ref().map(|v|v.current)}
        if name.eq_ignore_ascii_case(RESOURCE_NO_NI_OBJECTS){return self.resource_no_ni_objects.as_ref().map(|v|v.current)}
        if let Some(i)=quality_slot(name){return Some(self.texture_quality[i].current)}
        if name.eq_ignore_ascii_case(DATA_MISSING_RESOURCE_WARNING){return Some(self.data_missing_resource_warning.current)}
        Self::slot(name).map(|i|self.cells[i].current)
    }
    pub(crate) fn default_raw(&self,name:&str)->Option<u8> {
        if name.eq_ignore_ascii_case(RESOURCE_NO_NI_TEXTURES){return self.resource_no_ni_textures.as_ref().map(|v|v.source_snapshot)}
        if name.eq_ignore_ascii_case(RESOURCE_NO_NI_OBJECTS){return self.resource_no_ni_objects.as_ref().map(|v|v.source_snapshot)}
        if let Some(i)=quality_slot(name){return Some(self.texture_quality[i].source_snapshot)}
        if name.eq_ignore_ascii_case(DATA_MISSING_RESOURCE_WARNING){return Some(self.data_missing_resource_warning.source_snapshot)}
        Self::slot(name).map(|i|self.cells[i].source_snapshot)
    }
    pub(crate) fn failure(&self)->Option<&str> {self.failure.as_deref()}
    pub(crate) fn record_failure(&mut self,error:impl Into<String>) {
        if self.failure.is_none(){self.failure=Some(error.into())}
    }
    fn require(&self)->Result<(),String> {
        if let Some(error)=&self.failure {return Err(error.clone())}
        if !self.source_bound {return Err("VWorld UVar reset requires actual fresh source/profile ownership".into())}
        Ok(())
    }
    pub(crate) fn require_current(&self)->Result<(),String>{self.require()}
    pub(crate) fn files(&self)->&[SourceFile] {&self.source_files}

    /// Load the exact root wildcard set. These native declarations are
    /// Bool/type1. The seven flags5 cells accept only absent/false source values
    /// in this bounded scope: a positive override needs real EDITORDEV gating.
    /// Both allowed and denied assignments leave an already-zero cell at zero.
    /// CameraInformation flags16 is writable without that developer guard.
    pub(crate) fn bind_fresh_source(&mut self,root:&DataRoot,scope:SourceOverrides)->Result<(),String> {
        if scope!=SourceOverrides::ControlledNone {
            return Err("VWorld current UVar profile/console override owner is unbound".into())
        }
        if self.source_bound || self.cells!=Self::from_native_constructors().cells || self.resource_no_ni_textures.is_some() || self.resource_no_ni_objects.is_some() || self.texture_quality!=Self::from_native_constructors().texture_quality || self.data_missing_resource_warning!=Self::from_native_constructors().data_missing_resource_warning {
            return Err("VWorld fresh UVar binding requires its same fresh constructor cells".into())
        }
        let names=root.list_dir("");
        let ordered=native_enumerate("UVars*.tgi",&names);
        if !ordered.iter().any(|p|p.eq_ignore_ascii_case("UVars.tgi")) {
            return Err("VWorld root UVars.tgi source is absent".into())
        }
        for path in ordered {
            let bytes=root.read(&path).map_err(|e|e.to_string())?;
            let sha256=aa_rules::sources::sha256(&bytes);
            let doc=Document::parse_native(bytes,Limits::default()).map_err(|e|e.to_string())?;
            self.source_files.push(SourceFile {path:path.clone(),sha256});
            let mut named_seen=[false;2];
            let mut quality_seen=[false;2];
            let mut warning_seen=false;
            for item in &doc.items {
                let Item::Node(node)=item else {continue};
                if !doc.header_name(&node.header).is_some_and(|n|n.eq_ignore_ascii_case("Vars")){continue}
                if node.header.tokens.len()!=1 {return Err("VWorld UVar inheritance requires native source lookup ownership".into())}
                for item in &node.items {
                    let Item::Property(property)=item else {continue};
                    let Some(name)=doc.property_name(property)else {continue};
                    if name.eq_ignore_ascii_case(DATA_MISSING_RESOURCE_WARNING) {
                        if warning_seen{return Err("DataMissingResourceWarning duplicate source field requires native hash ownership".into())}warning_seen=true;
                        let typed=property.left.len()==2 && doc.text(property.left[0].span).is_some_and(|kind|kind.eq_ignore_ascii_case("flag")||kind.eq_ignore_ascii_case("bool"));
                        if !typed && property.left.len()!=1{return Err("DataMissingResourceWarning source requires its actual Bool type".into())}
                        let raw=doc.property_value(property).ok_or("DataMissingResourceWarning source value absent")?.trim();
                        self.data_missing_resource_warning.current=if raw.eq_ignore_ascii_case("true"){1}else if raw.eq_ignore_ascii_case("false"){0}else{return Err("DataMissingResourceWarning native source Bool conversion is unbound".into())};
                        continue
                    }
                    if let Some(i)=quality_slot(&name) {
                        if quality_seen[i]{return Err("Texture quality duplicate source field requires native hash ownership".into())}quality_seen[i]=true;
                        let typed=property.left.len()==2 && doc.text(property.left[0].span).is_some_and(|kind|kind.eq_ignore_ascii_case("flag")||kind.eq_ignore_ascii_case("bool"));
                        if !typed && property.left.len()!=1{return Err("Texture quality source requires its actual Bool type".into())}
                        let raw=doc.property_value(property).ok_or("Texture quality Bool source value absent")?.trim();
                        let value=if raw.eq_ignore_ascii_case("true"){1}else if raw.eq_ignore_ascii_case("false"){0}else{return Err("Texture quality native source Bool conversion is unbound".into())};
                        self.texture_quality[i].current=value;continue
                    }
                    if name.eq_ignore_ascii_case(RESOURCE_NO_NI_TEXTURES)||name.eq_ignore_ascii_case(RESOURCE_NO_NI_OBJECTS) {
                        let index=usize::from(name.eq_ignore_ascii_case(RESOURCE_NO_NI_OBJECTS));
                        let cell=if index==0{&mut self.resource_no_ni_textures}else{&mut self.resource_no_ni_objects};
                        //A duplicate same-name field in one source hash has
                        //different traversal semantics; do not invent its order.
                        if named_seen[index]{return Err(format!("{name} duplicate source field requires native hash ownership"))}
                        named_seen[index]=true;
                        let typed=property.left.len()==2 && doc.text(property.left[0].span)
                            .is_some_and(|kind|kind.eq_ignore_ascii_case("flag")||kind.eq_ignore_ascii_case("bool"));
                        if !typed && !(property.left.len()==1 && cell.is_some()) {
                            return Err(format!("{name} requires actual source-created Bool type"))
                        }
                        let raw=doc.property_value(property).ok_or("ResourceNoNiTextures source value absent")?.trim();
                        let value=if raw.eq_ignore_ascii_case("true"){1}else if raw.eq_ignore_ascii_case("false"){0}
                            else{return Err("ResourceNoNiTextures native source Bool conversion is unbound".into())};
                        if let Some(cell)=cell {cell.current=value}
                        else {*cell=Some(BoolCell {current:value,reset_default:value,source_snapshot:value})}
                        continue
                    }
                    let Some(slot)=Self::slot(&name)else {continue};
                    if property.left.len()!=2 || !doc.text(property.left[0].span)
                        .is_some_and(|kind|kind.eq_ignore_ascii_case("flag")||kind.eq_ignore_ascii_case("bool")||kind.eq_ignore_ascii_case("boolean")) {
                        return Err("VWorld UVar requires its native Bool declaration".into())
                    }
                    let value=doc.property_value(property).ok_or("VWorld UVar assignment has no value")?;
                    let value=if value.trim().eq_ignore_ascii_case("false"){0}
                        else if value.trim().eq_ignore_ascii_case("true"){1}
                        else {return Err("VWorld source Bool requires proven true/false literal scope".into())};
                    let metadata=registration::REGISTRATIONS[OWNED_BOOL_INDICES[slot]as usize];
                    if metadata.flags&2!=0 && value!=0 {
                        return Err("positive read-only DevMode source assignment requires actual native override writer".into())
                    }
                    if metadata.flags&5==5 && value!=0 {
                        return Err("positive editor UVar source assignment requires actual EDITORDEV gate".into())
                    }
                    self.cells[slot].current=value;
                }
            }
        }
        for cell in &mut self.cells {cell.source_snapshot=cell.current}
        if let Some(cell)=&mut self.resource_no_ni_textures {cell.source_snapshot=cell.current}
        if let Some(cell)=&mut self.resource_no_ni_objects {cell.source_snapshot=cell.current}
        for cell in &mut self.texture_quality{cell.source_snapshot=cell.current}
        self.data_missing_resource_warning.source_snapshot=self.data_missing_resource_warning.current;
        self.source_bound=true;Ok(())
    }
    /// Actual4B29B0/Bool setter projection. None never invents developer access.
    /// A denied write retains the old byte. CameraInformation has flags16.
    pub(crate) fn assign(&mut self,name:&str,value:u8,editor_dev:Option<bool>)->Result<(),String> {
        if name.eq_ignore_ascii_case(RESOURCE_NO_NI_OBJECTS){self.resource_no_ni_objects.as_mut().ok_or("ResourceNoNiObjects source-created allocation absent")?.current=value;return Ok(())}
        if name.eq_ignore_ascii_case(DATA_MISSING_RESOURCE_WARNING){self.data_missing_resource_warning.current=value;return Ok(())}
        if let Some(i)=quality_slot(name) {
            //Both actual named constructors use Bool flags16:4B29B0 has
            //no read-only/editor gate;4B3190 stores this raw byte only.
            self.texture_quality[i].current=value;return Ok(())
        }
        if name.eq_ignore_ascii_case(RESOURCE_NO_NI_TEXTURES) {
            //4B3190 through flags0 Bool writes the raw byte. No indexed slot,
            //read-only/editor gate or reconstructed source value is involved.
            self.resource_no_ni_textures.as_mut().ok_or("ResourceNoNiTextures source-created allocation absent")?.current=value;
            return Ok(())
        }
        let slot=Self::slot(name).ok_or("not one of the owned native view Bool cells")?;
        let metadata=registration::REGISTRATIONS[OWNED_BOOL_INDICES[slot]as usize];
        //4B29B6 tests bit2 before either application/editor gate. DevMode
        //is read-only to ordinary4B3190 assignments, including private UI
        //overrides. No current/default byte changes on this denied write.
        if metadata.flags&2!=0{return Ok(())}
        if metadata.flags&5==5 {
            match editor_dev {
                Some(false)=>return Ok(()),Some(true)=>{},
                None=>return Err("editor UVar write requires actual EDITORDEV gate".into()),
            }
        }
        self.cells[slot].current=value;Ok(())
    }
    pub(crate) fn reset(&mut self,mut session_f8:impl FnMut()->Result<u32,String>)->Result<ResetTrace,ResetFailure> {
        let mut trace=ResetTrace::default();
        if let Err(reason)=self.require(){return Err(ResetFailure {reason,trace})}
        for metadata in &registration::REGISTRATIONS {
            //All213 actual registrations are retained as immutable metadata.
            trace.events.push(ResetEvent::Lookup(metadata.index));
            if metadata.flags&5==5 && metadata.native_kind==1 {
                let slot=Self::slot(metadata.name).expect("all proven mutable indices are owned");
                self.cells[slot].current=0;trace.events.push(ResetEvent::Reset(metadata.index));
            }
            if metadata.index==44 {
                trace.events.push(ResetEvent::SessionRead);
                let first=session_f8().map_err(|reason|ResetFailure {reason,trace:trace.clone()})?;
                if first!=7 {
                    trace.events.push(ResetEvent::SessionRead);
                    let second=session_f8().map_err(|reason|ResetFailure {reason,trace:trace.clone()})?;
                    if second!=5 {
                        let slot=Self::slot(metadata.name).unwrap();
                        self.cells[slot].current=self.cells[slot].reset_default;
                        trace.events.push(ResetEvent::Reset(metadata.index));
                    }
                }
            }
        }
        Ok(trace)
    }
}

#[derive(Clone,Debug,PartialEq,Eq)]
pub enum ResetEvent {Lookup(u16),Reset(u16),SessionRead}
#[derive(Clone,Debug,Default,PartialEq,Eq)]
pub struct ResetTrace {pub events:Vec<ResetEvent>}
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct ResetFailure {pub reason:String,pub trace:ResetTrace}

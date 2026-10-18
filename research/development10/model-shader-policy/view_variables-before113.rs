//! Native43F156..43F1B2 reset over the213 registered UVars. Only eight
//! reset Bool cells are reached; RendererPreRequestAllVThings also has one
//! canonical Bool cell for native45B530. The other204 entries are lookup
//! metadata, not duplicated current values. CameraInformation is stored here
//! once and borrowed by the existing UVars/HUD/options accessors.
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
pub const OWNED_BOOL_INDICES:[u16;9]=[22,24,25,27,29,44,100,101,108];

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
    cells:[BoolCell;9],
    source_files:Vec<SourceFile>,
    failure:Option<String>,
    source_bound:bool,
}
impl Default for State {fn default()->Self {Self::from_native_constructors()}}
impl State {
    ///4017D0 from the real registration capture: the eight reset targets
    ///start0/0; RendererPreRequestAllVThings starts1/1. This is not by itself
    ///a source/profile completion certificate.
    pub(crate) fn from_native_constructors()->Self {
        let mut cells=[BoolCell {current:0,reset_default:0,source_snapshot:0};9];
        cells[8]=BoolCell {current:1,reset_default:1,source_snapshot:1};
        Self {cells,
            source_files:Vec::new(),failure:None,source_bound:false}
    }
    fn slot(name:&str)->Option<usize> {
        OWNED_BOOL_INDICES.iter().position(|&i|registration::REGISTRATIONS[i as usize].name.eq_ignore_ascii_case(name))
    }
    pub(crate) fn contains(name:&str)->bool {Self::slot(name).is_some()}
    pub(crate) fn raw(&self,name:&str)->Option<u8> {Self::slot(name).map(|i|self.cells[i].current)}
    pub(crate) fn default_raw(&self,name:&str)->Option<u8> {Self::slot(name).map(|i|self.cells[i].source_snapshot)}
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

    /// Load the exact root wildcard set. These nine native declarations are
    /// Bool/type1. The seven flags5 cells accept only absent/false source values
    /// in this bounded scope: a positive override needs real EDITORDEV gating.
    /// Both allowed and denied assignments leave an already-zero cell at zero.
    /// CameraInformation flags16 is writable without that developer guard.
    pub(crate) fn bind_fresh_source(&mut self,root:&DataRoot,scope:SourceOverrides)->Result<(),String> {
        if scope!=SourceOverrides::ControlledNone {
            return Err("VWorld current UVar profile/console override owner is unbound".into())
        }
        if self.source_bound || self.cells!=Self::from_native_constructors().cells {
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
            for item in &doc.items {
                let Item::Node(node)=item else {continue};
                if !doc.header_name(&node.header).is_some_and(|n|n.eq_ignore_ascii_case("Vars")){continue}
                if node.header.tokens.len()!=1 {return Err("VWorld UVar inheritance requires native source lookup ownership".into())}
                for item in &node.items {
                    let Item::Property(property)=item else {continue};
                    let Some(name)=doc.property_name(property)else {continue};
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
                    if metadata.flags&5==5 && value!=0 {
                        return Err("positive editor UVar source assignment requires actual EDITORDEV gate".into())
                    }
                    self.cells[slot].current=value;
                }
            }
        }
        for cell in &mut self.cells {cell.source_snapshot=cell.current}
        self.source_bound=true;Ok(())
    }
    /// Actual4B29B0/Bool setter projection. None never invents developer access.
    /// A denied write retains the old byte. CameraInformation has flags16.
    pub(crate) fn assign(&mut self,name:&str,value:u8,editor_dev:Option<bool>)->Result<(),String> {
        let slot=Self::slot(name).ok_or("not one of the owned native view Bool cells")?;
        let metadata=registration::REGISTRATIONS[OWNED_BOOL_INDICES[slot]as usize];
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

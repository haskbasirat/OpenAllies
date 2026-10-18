//! Source-bound40BC50 GENERIC_EVENT/ordinal15,40B7C0 payload and40B5C0 list
//! publication over the SAME Ui.messages/Ui.uvars owners. The legacy widget
//! helper is explicitly a display laboratory; this path calls every reached
//! time/View/diagnostic child and preserves stores on child failure.
use crate::{message_log::{Message,MessageSystem},uvars::UVars};
use aa_numeric::{Binary,Precision};
use aa_rules::{actors::ActorCatalog,native::TypedValue,resources::{Provenance,SourceIdentity}};
use std::sync::Weak;
pub type Result<T>=std::result::Result<T,String>;

///497BA0 registers this fixed numeric slot. Authored root order and a
///HashMap's iteration order are never native MessageType indices.
pub const GENERIC_EVENT_ORDINAL:u32=15;
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Header{ordinal:u32,timestamp_bits:u32,view_word14:u32,text:Vec<u16>}
impl Header{
    pub fn ordinal(&self)->u32{self.ordinal}
    pub fn timestamp_bits(&self)->u32{self.timestamp_bits}
    pub fn view_word14(&self)->u32{self.view_word14}
    pub fn text_utf16(&self)->&[u16]{&self.text}
}
#[derive(Debug)]
pub(crate) struct Binding{uvars:Weak<()>,source:GenericSource,integrity:String}
///Immutable send-field projection of one real predeclared MessageType.
///This is not a claim that TextureResource allocation/GPU upload completed.
#[derive(Debug)]
struct GenericSource{sources:Vec<SourceIdentity>,definition:Provenance,filter:String}
impl GenericSource{
    fn compile(catalog:&ActorCatalog,precision:Precision)->Result<Self>{
        let registry=catalog.native_data_registry("MessageType").ok_or("native MessageType manager absent")?;
        if registry.mode!=Some(2)||!registry.translations().is_empty(){return Err("MessageType mode/translated array identity is unsupported".into())}
        let candidates:Vec<_>=registry.entries.iter().filter(|e|e.ids.eq_ignore_ascii_case("GENERIC_EVENT")).collect();
        if candidates.len()!=1{return Err("native GENERIC_EVENT source is not unique".into())}
        let entry=candidates[0];
        if !registry.find("GENERIC_EVENT").is_some_and(|e|std::ptr::eq(e,entry)){
            return Err("MessageType public projection differs from private canonical registry".into())
        }
        let node=catalog.native_data_node(entry).map_err(|e|e.to_string())?;
        if node.sources().first()!=Some(&entry.source){return Err("MessageType canonical source path differs from its registered node".into())}
        let string=|name:&str|->Result<String>{match node.field(name,precision).map_err(|e|e.to_string())?{
            Some(TypedValue::String{value,..})=>Ok(value),_=>Err(format!("MessageType {name} typed source String absent")),}};
        if !string("IDS")?.eq_ignore_ascii_case("GENERIC_EVENT"){return Err("MessageType source IDS differs from ordinal15".into())}
        let filter=string("filter")?;
        if !filter.eq_ignore_ascii_case("UIMessageFilterGeneric"){return Err("ordinal15 filter requires an additional actual current UVar binding".into())}
        if !string("message_sound")?.is_empty(){return Err("ordinal15 positive Audio definition/resource binding is required".into())}
        if !catalog.sources.iter().any(|s|s.path.eq_ignore_ascii_case(&entry.source.path)&&s.sha256==entry.source.sha256){
            return Err("MessageType source is outside the retained source universe".into())
        }
        Ok(Self{sources:catalog.sources.clone(),definition:entry.source.clone(),filter})
    }
}
///Required current process owners. A present VWorld returns its actual+8C;
///only an actual null VWorld may return None. Rust's exclusive MessageSystem
///loan supplies list exclusion; this does not implement the Win32 lock ABI.
pub trait Services{
    fn milliseconds_59aba0(&mut self)->Result<u32>;
    fn active_view_word_8c(&mut self)->Result<Option<u32>>;
    ///404F60(channel B38B70, MessageSystem.cpp line398), after list/id stores.
    fn message_diagnostic(&mut self,message:&Message)->Result<()>;
    fn application_kind_b396c0(&mut self)->Result<u32>;
}
fn timestamp(milliseconds:u32,precision:Precision)->Result<u32>{
    //40B7FB FILD signed32 then40B801 FADD binary32 2^32 for unsigned values.
    let mut value=Binary::from_i32(milliseconds as i32);
    if milliseconds&0x80000000!=0{value=value.add(&Binary::from_f32_bits(0x4f800000).map_err(|e|e.to_string())?,precision).map_err(|e|e.to_string())?;}
    value.mul(&Binary::from_f32_bits(0x3a83126f).map_err(|e|e.to_string())?,precision)
        .and_then(|v|v.store_f32()).map_err(|e|e.to_string())
}
fn integrity(system:&MessageSystem)->String{
    //An immutable checkpoint detects writes through the public display API;
    //it is not a second mutable list/id/generation owner.
    let mut bytes=Vec::new();
    for word in[system.generation,system.last,system.count_30]{bytes.extend_from_slice(&word.to_le_bytes())}
    for message in &system.messages{
        bytes.extend_from_slice(&message.id.to_le_bytes());
        for value in[&message.kind,&message.text]{bytes.extend_from_slice(&(value.len()as u64).to_le_bytes());bytes.extend_from_slice(value.as_bytes());}
        bytes.push(u8::from(message.location.is_some()));
        if let Some((x,y))=message.location{bytes.extend_from_slice(&x.to_bits().to_le_bytes());bytes.extend_from_slice(&y.to_bits().to_le_bytes())}
        bytes.push(u8::from(message.native.is_some()));
        if let Some(header)=&message.native{
            for word in[header.ordinal,header.timestamp_bits,header.view_word14]{bytes.extend_from_slice(&word.to_le_bytes())}
            bytes.extend_from_slice(&(header.text.len()as u64).to_le_bytes());for unit in &header.text{bytes.extend_from_slice(&unit.to_le_bytes())}
        }
    }
    aa_rules::sources::sha256(&bytes)
}
impl MessageSystem{
    ///Fresh40B510 fields are created by this same MessageSystem constructor.
    ///Binding cannot promote an old display list or a cloned option draft.
    pub fn bind_native_generic(&mut self,catalog:&ActorCatalog,source_precision:Precision,uvars:&UVars)->Result<()>{
        if !self.native_constructor||self.native.is_some()||!self.messages.is_empty()||self.generation!=0||self.last!=0||self.count_30!=0{
            return Err("native MessageSystem binding requires the same fresh constructor/list fields".into())
        }
        uvars.require_message_source()?;
        let source=GenericSource::compile(catalog,source_precision)?;
        self.native=Some(Binding{uvars:uvars.message_owner(),source,integrity:integrity(self)});Ok(())
    }
    pub fn native_generic_source(&self)->Result<(&[SourceIdentity],&Provenance)>{
        let binding=self.native.as_ref().ok_or("native MessageSystem source is unbound")?;
        Ok((&binding.source.sources,&binding.source.definition))
    }
    pub fn native_count(&self)->Result<u32>{self.require_native_integrity()?;Ok(self.count_30)}
    fn require_native_integrity(&self)->Result<()>{
        let binding=self.native.as_ref().ok_or("native MessageSystem source is unbound")?;
        if binding.integrity!=integrity(self){return Err("native MessageSystem fields changed outside its same-owner publication path".into())}Ok(())
    }
    ///Consumes an already-formatted native CString payload at40BC9D's return;
    ///no printf conversion or lossy UTF16 conversion is used for native data.
    ///The returned bool is40B5C0's accepted/filter result, not game readiness.
    pub fn send_native_generic(&mut self,uvars:&UVars,text:&[u16],services:&mut impl Services,precision:Precision)->Result<bool>{
        self.require_native_integrity()?;
        let binding=self.native.as_ref().expect("checked");
        if binding.uvars.strong_count()==0||!Weak::ptr_eq(&binding.uvars,&uvars.message_owner()){
            return Err("native message requires the same retained Ui.uvars owner".into())
        }
        let result=(||{
            let timestamp_bits=timestamp(services.milliseconds_59aba0()?,precision)?;
            let view_word14=services.active_view_word_8c()?.unwrap_or(0);
            //Construction (including both process reads) precedes filtering.
            let filter=&self.native.as_ref().expect("checked").source.filter;
            if !filter.eq_ignore_ascii_case("UIMessageFilterGeneric"){return Err("native message current filter owner absent".into())}
            if uvars.message_filter_generic_current()?==0{return Ok(false)}
            self.messages.try_reserve(1).map_err(|e|e.to_string())?;
            self.last=self.last.wrapping_add(1);
            self.messages.push_front(Message{id:self.last,kind:"GENERIC_EVENT".into(),text:String::from_utf16_lossy(text),location:None,
                native:Some(Header{ordinal:GENERIC_EVENT_ORDINAL,timestamp_bits,view_word14,text:text.to_vec()})});
            services.message_diagnostic(self.messages.front().expect("just inserted"))?;
            //Native reads AppKind even though proved ordinal15 Audio is NULL.
            let _=services.application_kind_b396c0()?;
            let maximum=uvars.message_maximum_lines_current()?;
            if (self.count_30 as i32)<maximum{self.count_30=self.count_30.wrapping_add(1)}
            else{self.messages.pop_back();}
            Ok(true)
        })();
        let checkpoint=integrity(self);self.native.as_mut().expect("retained").integrity=checkpoint;
        result
    }
}

#[cfg(test)]
#[path="native_messages_tests.rs"]
mod tests;

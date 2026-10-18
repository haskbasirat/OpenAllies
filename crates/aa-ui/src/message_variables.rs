//! Immutable fresh source receipt for native UVar67/78. Current/default
//!values remain solely in UVars' existing maps; this owns no current mirror.
use crate::{fs::DataRoot,view_variables::{SourceFile,SourceOverrides}};
use aa_rules::{native::index::native_enumerate,tgi::{Document,Item,Limits}};
#[derive(Clone,Debug,Default)]
pub(crate) struct Receipt{files:Vec<SourceFile>,bound:bool,failure:Option<String>}
pub(crate) struct SourceValues{pub maximum:i32,pub filter:u8}
impl Receipt{
    pub(crate) fn bind(&mut self,root:&DataRoot,scope:SourceOverrides)->Result<SourceValues,String>{
        if self.bound{return Err("message UVar source already bound".into())}
        if scope!=SourceOverrides::ControlledNone{return Err("message current UVar profile/console owner is unbound".into())}
        let ordered=native_enumerate("UVars*.tgi",&root.list_dir(""));
        if !ordered.iter().any(|p|p.eq_ignore_ascii_case("UVars.tgi")){return Err("message UVars.tgi source is absent".into())}
        let mut out=SourceValues{maximum:50,filter:1};
        for path in ordered{
            let bytes=root.read(&path).map_err(|e|e.to_string())?;let hash=aa_rules::sources::sha256(&bytes);
            let doc=Document::parse_native(bytes,Limits::default()).map_err(|e|e.to_string())?;
            self.files.push(SourceFile{path,sha256:hash});
            for item in &doc.items{
                let Item::Node(n)=item else{continue};
                if !doc.header_name(&n.header).is_some_and(|s|s.eq_ignore_ascii_case("Vars")){continue}
                if n.header.tokens.len()!=1{return Err("message UVar source inheritance requires native loader".into())}
                for item in &n.items{
                    let Item::Property(p)=item else{continue};let Some(name)=doc.property_name(p)else{continue};
                    if !name.eq_ignore_ascii_case("MessageLogMaximumLines")&&!name.eq_ignore_ascii_case("UIMessageFilterGeneric"){continue}
                    if p.left.len()!=2{return Err("message UVar requires its native typed declaration".into())}
                    let kind=doc.text(p.left[0].span).ok_or("message UVar type missing")?;
                    let raw=doc.property_value(p).ok_or("message UVar assignment missing")?.trim();
                    if name.eq_ignore_ascii_case("MessageLogMaximumLines"){
                        if !kind.eq_ignore_ascii_case("int")&&!kind.eq_ignore_ascii_case("integer"){return Err("message UVar67 requires native Int source".into())}
                        out.maximum=raw.parse().map_err(|_|"message UVar67 native nonliteral Int conversion required")?;
                    }else{
                        if !["flag","bool","boolean"].iter().any(|s|kind.eq_ignore_ascii_case(s)){return Err("message UVar78 requires native Bool source".into())}
                        out.filter=if raw.eq_ignore_ascii_case("true"){1}else if raw.eq_ignore_ascii_case("false"){0}else{return Err("message UVar78 native nonliteral Bool conversion required".into())};
                    }
                }
            }
        }
        self.bound=true;Ok(out)
    }
    pub(crate) fn fail(&mut self,e:String){if self.failure.is_none(){self.failure=Some(e)}}
    pub(crate) fn require(&self)->Result<(),String>{
        if let Some(e)=&self.failure{return Err(e.clone())}
        if !self.bound{return Err("message current UVars source not bound".into())}Ok(())
    }
    pub(crate) fn files(&self)->&[SourceFile]{&self.files}
}

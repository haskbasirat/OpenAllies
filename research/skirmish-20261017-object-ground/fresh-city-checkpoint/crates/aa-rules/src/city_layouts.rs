//!4C05F0's immutable CityLayout rows after475230 deferred-reference binding.
use crate::{actors::ActorCatalog,native::{Error,Result,TypedValue}};
use aa_numeric::Precision;
#[derive(Clone,Debug,PartialEq,Eq)]
pub struct Object {
    pub definition_ids:String,
    pub offset:[u32;2],
    pub orientation:[u32;2],
}
impl ActorCatalog {
    pub fn native_city_layouts(&self,id:&str,pc:Precision)->Result<Option<Vec<Vec<Object>>>> {
        let actor=self.resolved_actor(id)?;
        let Some(city)=actor.child("CityComponent")? else{return Ok(None)};
        let mut out=Vec::new();
        for layout in city.children("Layout")? {
            let mut rows=Vec::new();
            for row in layout.children("Object")? {
                let Some(TypedValue::String{value,..})=row.field("object_ids",pc)? else{return Err(Error::Unsupported("City Object object_ids missing or wrong type".into()))};
                if value.is_empty(){return Err(Error::Unsupported("City layout missing object IDS".into()))}
                let pair=|name:&str|->Result<[u32;2]>{match row.field(name,pc)? {
                    Some(TypedValue::Point{bits,..})=>bits.try_into().map_err(|_|Error::Unsupported(format!("City Object {name} must be point2"))),
                    _=>Err(Error::Unsupported(format!("City Object {name} missing or wrong type"))),
                }};
                rows.push(Object{definition_ids:value,offset:pair("offset")?,orientation:pair("orientation")?});
            }
            if rows.is_empty(){return Err(Error::Unsupported("City layout requires at least one object (4C0790)".into()))}
            //473AD0 prepends each deferred object;475230 walks that list and
            //4C05D0/4C0560 append into this Layout in reverse authored order.
            rows.reverse();
            out.push(rows);
        }
        if out.is_empty(){return Err(Error::Unsupported("City requires at least one Layout (468493)".into()))}
        Ok(Some(out))
    }
}

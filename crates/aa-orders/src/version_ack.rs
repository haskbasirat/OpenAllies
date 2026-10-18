//! Direct766CF0 ->751FC0 acknowledgement. This is not a request/decree Shape.
use crate::{Error,Result};

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Diagnostic {
    MissingOrderingPlayer, //751FE3, lineB46
    MissingOrderingClient, //751FFD, lineB4B
    OrderingPlayerBecameNull, //752038/752070, lineA13
    Accepted, //7520A9, lineB52, format%s
}
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Outcome {MissingPlayer,MissingClient,Acknowledged}

///All fields and effects are reached lazily on the actual Admin/Session/client.
pub trait World {
    type Player;
    type Client;
    fn ordering_player(&mut self)->Result<Option<Self::Player>>;
    fn player_client(&mut self,player:&Self::Player)->Result<Option<Self::Client>>;
    fn store_acknowledged(&mut self,client:&Self::Client)->Result<()>;
    fn client_description(&mut self,client:&Self::Client)->Result<Vec<u16>>;
    fn diagnostic(&mut self,kind:Diagnostic,description:Option<&[u16]>)->Result<()>;
}
pub fn run<W:World>(w:&mut W)->Result<Outcome> {
    let Some(player)=w.ordering_player()? else {
        w.diagnostic(Diagnostic::MissingOrderingPlayer,None)?;
        return Ok(Outcome::MissingPlayer);
    };
    if w.player_client(&player)?.is_none() {
        w.diagnostic(Diagnostic::MissingOrderingClient,None)?;
        return Ok(Outcome::MissingClient);
    }
    let client=required_current_client(w)?;
    w.store_acknowledged(&client)?; //752062 persists before7515A0/log errors.
    let client=required_current_client(w)?;
    let text=w.client_description(&client)?;
    w.diagnostic(Diagnostic::Accepted,Some(&text))?;
    Ok(Outcome::Acknowledged)
}
fn required_current_client<W:World>(w:&mut W)->Result<W::Client> {
    let Some(player)=w.ordering_player()? else {
        w.diagnostic(Diagnostic::OrderingPlayerBecameNull,None)?;
        return Err(Error::SchemaContext("751FC0 would dereference a null ordering client after diagnostic"));
    };
    w.player_client(&player)?.ok_or(Error::SchemaContext("751FC0 requires the actual nonnull client before6C write/description"))
}

///7515A0's source reads are ordered:765E00, name766380, optional765E00/
///Color+10, player20. Sources remain the same retained fields throughout.
pub trait DescriptionWorld {
    fn has_current_kingdom(&mut self)->Result<bool>;
    fn player_name(&mut self)->Result<Vec<u16>>;
    fn kingdom_color_name(&mut self)->Result<Vec<u16>>;
    fn player_id(&mut self)->Result<u32>;
}
pub fn describe(w:&mut impl DescriptionWorld)->Result<Vec<u16>> {
    let has=w.has_current_kingdom()?;
    let name=w.player_name()?;
    let color=if has{Some(w.kingdom_color_name()?)}else{None};
    let id=w.player_id()?;
    format_description(&name,id,color.as_deref())
}
pub fn c_string(units:&[u16])->&[u16] {
    &units[..units.iter().position(|&c|c==0).unwrap_or(units.len())]
}
///VC7.1 405D60 [%s,%d] / [%s,%d,%s], preserving raw UTF16 before NUL.
pub fn format_description(name:&[u16],id:u32,color:Option<&[u16]>)->Result<Vec<u16>> {
    let name=c_string(name);let color=color.map(c_string);
    let number=(id as i32).to_string();
    let size=name.len().checked_add(color.map_or(0,<[u16]>::len)).and_then(|n|n.checked_add(number.len()+4)).ok_or(Error::Allocation)?;
    let mut out=Vec::new();out.try_reserve(size).map_err(|_|Error::Allocation)?;
    out.push(b'[' as u16);out.extend_from_slice(name);out.push(b',' as u16);out.extend(number.encode_utf16());
    if let Some(color)=color {out.push(b',' as u16);out.extend_from_slice(color);}
    out.push(b']' as u16);Ok(out)
}

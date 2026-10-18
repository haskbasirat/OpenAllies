//!Native4E3C70 selection and Kingdom30C/310 ordered list mutation.
//!The same typed list is also read by4E3AB0 and freed by4E57C0.
use aa_actors::ActorId;
type Result<T>=std::result::Result<T,String>;
pub trait World{
    fn required_city_158(&mut self)->Result<ActorId>;
    ///Actual current generation +nonnull pointer, not a saved record census.
    fn resolve_current(&mut self,id:ActorId)->Result<Option<ActorId>>;
    ///Actual definition virtual24 (ObjectData462E50 reads source byte34).
    fn definition_24(&mut self,id:ActorId)->Result<bool>;
    fn current_actor(&self)->ActorId;
    fn has_city_d4(&mut self)->Result<bool>;
    fn list_30c(&mut self)->Result<&mut Vec<ActorId>>;
}
pub fn update(world:&mut impl World,add:bool)->Result<()>{
    let required=world.required_city_158()?;
    let mut selected=None;
    if let Some(actor)=world.resolve_current(required)?{
        if world.definition_24(actor)?{
            //Native rereads Actor158 and its current generation after vf24.
            let current=world.required_city_158()?;
            selected=world.resolve_current(current)?;
        }
    }
    let actor=if let Some(actor)=selected{actor}else{
        if !world.has_city_d4()?{return Ok(())}
        world.current_actor()
    };
    let list=world.list_30c()?;
    if add{
        if list.contains(&actor){return Ok(())}
        list.try_reserve(1).map_err(|_|"4D0900 Kingdom30C list-node allocation failed")?;
        list.insert(0,actor);
    }else if let Some(index)=list.iter().position(|&id|id==actor){
        list.remove(index); //4F1430 removes only the first equal node.
    }
    Ok(())
}
///4E3AB0's signed DWORD increments, including its native wrapping domain.
pub fn count(list:&[ActorId])->i32{list.len()as u32 as i32}
#[cfg(test)]#[path="kingdom_ownership_track_tests.rs"]mod tests;

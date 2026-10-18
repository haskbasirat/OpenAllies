//! Native4E4980 automatic number assignment over the same staged Kingdom table.
//! NameList4F4A20 and string publication4D7F40 are separate operations.
use crate::{actor_world::{ObjectKey,Source},map_replacement_queue::DefinitionRef,restoration::Staged};
type Result<T>=std::result::Result<T,String>;
fn err(e:impl std::fmt::Display)->String{e.to_string()}
///The caller has passed4D6F10's actual source numbered gate. This function
///rereads current virtual108 owner and keeps table writes before Actor170.
pub fn assign_automatic(staged:&mut Staged,key:ObjectKey,definition:&DefinitionRef)->Result<()> {
    let id=staged.actors.id(key).ok_or("4E4980 current Actor GTID absent")?;
    let owner=staged.actors.ordinary_owner(id).map_err(err)?.ok_or("4E4980 actual numbered owner is null")?;
    let entry=staged.definitions.require_usable(definition.token)?;
    let current=&staged.actors.object(key).map_err(err)?.definition.proof.id;
    if entry.key.domain!="actordefinition" || !entry.key.name.eq_ignore_ascii_case(current)
        || !definition.canonical_ids.eq_ignore_ascii_case(current){
        return Err("4E4980 current ActorData/Bank token differs".into());
    }
    let owner_key=staged.actors.key(owner).ok_or("4E4980 current owner GTID is stale")?;
    let entity=staged.actors.entity_mut(owner_key).map_err(err)?;
    if entity.runtime.as_ref().map(|r|r.definition.native_class)!=Some(4){return Err("4E4980 receiver is not actual Kingdom class4".into());}
    let Source::Special(s)=&mut entity.source else{return Err("4E4980 actual Kingdom derived allocation absent".into());};
    let shell=s.derived.as_mut().ok_or("4E4980 actual Kingdom derived fields absent")?;
    if shell.definition_counts.is_none(){
        //Only the actual untouched4E5370 empty table may migrate here. A
        //missing saved owner is not silently replaced by an empty table.
        if shell.words.get(&0x360)!=Some(&10) || shell.words.get(&0x364)!=Some(&0)
            || shell.bytes.get(&0x368)!=Some(&1)
            || !shell.indexed_words.get(&0x35c).is_some_and(|v|v.len()==10 && v.iter().all(|&x|x==0)){
            return Err("4E4980 actual Kingdom35C table owner unbound".into());
        }
    }
    let value=shell.definition_counts_mut().assign(definition.token).map_err(err)?;
    //4CF1A0 is exactly one store. The increment/insert above is not rolled
    //back if a later current Actor field loan fails.
    staged.actors.object_mut(key).map_err(err)?.actor.as_mut()
        .ok_or("4CF1A0 current Actor fields absent")?.words.insert(0x170,value);
    Ok(())
}

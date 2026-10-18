from pathlib import Path
import re
p=Path('crates/aa-economy/src/kingdom_registry.rs');s=p.read_text();start=s.index('pub fn add_actor(');brace=s.index('{',start);depth=1;end=brace+1
while depth:
 if s[end]=='{':depth+=1
 elif s[end]=='}':depth-=1
 end+=1
body=s[brace+1:end-1].replace('world.', 'w.')
body=re.sub(r'    registry\s*\.class_lists\s*\.entry\(a.class_index\)\s*\.or_default\(\)\s*\.insert\(0, actor\);','    w.class_insert_head(a.class_index, actor)?;',body)
for field,name in [('structures','Structures'),('settlements','Settlements'),('hqs','Hqs'),('organizations','Organizations')]:body=body.replace(f'registry.{field}.push(actor);',f'w.list_push(ActorList::{name}, actor)?;')
body=body.replace('registry.organizations.len()', 'w.list_len(ActorList::Organizations)?')
body=re.sub(r'let organization = \*registry\s*\.organizations\s*\.get\(index\)\s*\.ok_or\(EconomyError::ResourceIndex\)\?;', 'let organization = w.list_at(ActorList::Organizations, index)?;',body)
body=body.replace('!registry.supply.contains(&actor)','!w.supply_contains(actor)?').replace('registry.supply.insert(0, actor);','w.supply_insert_head(actor)?;')
body=body.replace('actor_completed(registry, actor, world)?;', 'completed_stored(actor, w)?;')
body=body.replace('registry.capital != Some(actor)','w.capital()? != Some(actor)').replace('if let Some(previous) = registry.capital','if let Some(previous) = w.capital()?').replace('registry.capital = Some(actor);','w.set_capital(actor)?;')
body=body.replace('registry.word_190 == 0','w.word_190()? == 0').replace('count_actor(registry, actor, world)?;','count_stored(actor, w)?;')
body=body.replace('!registry.flares.contains(&actor)','!w.list_contains(ActorList::Flares, actor)?').replace('registry.flares.insert(0, actor);','w.list_insert_head(ActorList::Flares, actor)?;').replace('registry.flares.len()', 'w.list_len(ActorList::Flares)?')
body=body.replace('for &flare in &registry.flares {','for index in 0..w.list_len(ActorList::Flares)? {\n                let flare = w.list_at(ActorList::Flares, index)?;')
assert 'registry.' not in body and 'world' not in body,body
trait=s[s.index('pub trait RegistryWorld {'):s.index('\nfn bit(')]
methods=re.findall(r'    fn (\w+)\s*\((.*?)\)\s*->\s*(Result<.*?>)\s*;',trait,re.S)
forward=[]
for name,args,result in methods:
 clean=' '.join(args.split());rest=clean.split(',',1)[1].strip()if ','in clean else '';values=[a.split(':')[0].strip()for a in rest.split(',')if a.strip()]
 forward.append(f'    fn {name}({clean}) -> {" ".join(result.split())} {{ self.world.{name}({", ".join(values)}) }}')
assert len(methods)==21
new='''//! Native4E6180 over actual borrowed storage, with no detached Registry copy.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorList {Structures,Settlements,Hqs,Organizations,Flares}
/// Storage and effects share one owner. Each required access observes current
/// fields, including earlier list writes before a required child stops.
pub trait StoredRegistryWorld: RegistryWorld {
 fn class_insert_head(&mut self,class_index:u32,actor:u32)->Result<(),EconomyError>;
 fn list_push(&mut self,list:ActorList,actor:u32)->Result<(),EconomyError>;
 fn list_insert_head(&mut self,list:ActorList,actor:u32)->Result<(),EconomyError>;
 fn list_len(&self,list:ActorList)->Result<usize,EconomyError>;
 fn list_at(&self,list:ActorList,index:usize)->Result<u32,EconomyError>;
 fn list_contains(&self,list:ActorList,actor:u32)->Result<bool,EconomyError>;
 fn supply_contains(&self,actor:u32)->Result<bool,EconomyError>;
 fn supply_insert_head(&mut self,actor:u32)->Result<(),EconomyError>;
 fn increment_counted(&mut self)->Result<(),EconomyError>;
 fn capital(&self)->Result<Option<u32>,EconomyError>;
 fn set_capital(&mut self,actor:u32)->Result<(),EconomyError>;
 fn word_190(&self)->Result<u32,EconomyError>;
}
fn count_stored(actor:u32,w:&mut impl StoredRegistryWorld)->Result<(),EconomyError>{
 let flags=w.actor(actor)?.flags8;
 if flags&FLAG_COUNTED==0 && w.counted(actor)?{w.increment_counted()?;w.set_flags8(actor,flags|FLAG_COUNTED)?;}
 Ok(())
}
fn completed_stored(actor:u32,w:&mut impl StoredRegistryWorld)->Result<(),EconomyError>{
 count_stored(actor,w)?;
 if w.statistics_present()? {w.statistics_track(actor,true)?;if w.statistics_classified(actor)?{w.add_score(14,0x3f800000)?;}}
 w.ownership_track(actor,true)
}
/// Native AddActor storage order, shared with the existing Registry API.
pub fn add_actor_stored(actor:u32,w:&mut impl StoredRegistryWorld)->Result<(),EconomyError>{
'''+body+'''
}
struct Joined<'a,W>{registry:&'a mut KingdomRegistry,world:&'a mut W}
impl<W:RegistryWorld> RegistryWorld for Joined<'_,W>{
'''+ '\n'.join(forward)+'''
}
impl<W:RegistryWorld> Joined<'_,W>{
 fn list(&self,l:ActorList)->&Vec<u32>{match l{ActorList::Structures=>&self.registry.structures,ActorList::Settlements=>&self.registry.settlements,ActorList::Hqs=>&self.registry.hqs,ActorList::Organizations=>&self.registry.organizations,ActorList::Flares=>&self.registry.flares}}
 fn list_mut(&mut self,l:ActorList)->&mut Vec<u32>{match l{ActorList::Structures=>&mut self.registry.structures,ActorList::Settlements=>&mut self.registry.settlements,ActorList::Hqs=>&mut self.registry.hqs,ActorList::Organizations=>&mut self.registry.organizations,ActorList::Flares=>&mut self.registry.flares}}
}
impl<W:RegistryWorld> StoredRegistryWorld for Joined<'_,W>{
 fn class_insert_head(&mut self,c:u32,a:u32)->Result<(),EconomyError>{self.registry.class_lists.entry(c).or_default().insert(0,a);Ok(())}
 fn list_push(&mut self,l:ActorList,a:u32)->Result<(),EconomyError>{self.list_mut(l).push(a);Ok(())}
 fn list_insert_head(&mut self,l:ActorList,a:u32)->Result<(),EconomyError>{self.list_mut(l).insert(0,a);Ok(())}
 fn list_len(&self,l:ActorList)->Result<usize,EconomyError>{Ok(self.list(l).len())}
 fn list_at(&self,l:ActorList,i:usize)->Result<u32,EconomyError>{self.list(l).get(i).copied().ok_or(EconomyError::ResourceIndex)}
 fn list_contains(&self,l:ActorList,a:u32)->Result<bool,EconomyError>{Ok(self.list(l).contains(&a))}
 fn supply_contains(&self,a:u32)->Result<bool,EconomyError>{Ok(self.registry.supply.contains(&a))}
 fn supply_insert_head(&mut self,a:u32)->Result<(),EconomyError>{self.registry.supply.insert(0,a);Ok(())}
 fn increment_counted(&mut self)->Result<(),EconomyError>{self.registry.counted=self.registry.counted.wrapping_add(1);Ok(())}
 fn capital(&self)->Result<Option<u32>,EconomyError>{Ok(self.registry.capital)}
 fn set_capital(&mut self,a:u32)->Result<(),EconomyError>{self.registry.capital=Some(a);Ok(())}
 fn word_190(&self)->Result<u32,EconomyError>{Ok(self.registry.word_190)}
}
pub(super) fn add_compat<W:RegistryWorld>(registry:&mut KingdomRegistry,actor:u32,world:&mut W)->Result<(),EconomyError>{add_actor_stored(actor,&mut Joined{registry,world})}
'''
Path('crates/aa-economy/src/kingdom_registry_storage.rs').write_text(new)
s=s[:brace+1]+'\n    storage::add_compat(registry, actor, world)\n'+s[end-1:]
pos=s.index('pub const CITY_CLASS');s=s[:pos]+'#[path = "kingdom_registry_storage.rs"]\nmod storage;\npub use storage::{ActorList,StoredRegistryWorld,add_actor_stored};\n\n'+s[pos:];p.write_text(s)

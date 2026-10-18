//!8C72D0/8C7190 hidden-tree pruning for the admitted static node classes.
//!The actual adapter rejects controller/skin/SwitchNode cases before calling.
//!Node allocations remain in their typed owner; this makes no native free or
//!reference-count claim. Only actual parent/list membership is changed.
use std::collections::BTreeSet;
type Result<T>=std::result::Result<T,String>;
pub(super) trait Tree{
 fn parent(&self,id:usize)->Option<usize>;
 fn children(&self,id:usize)->&[Option<usize>];
 fn is_node(&self,id:usize)->bool;
 fn hidden(&self,id:usize)->bool;
 fn selection_box(&self,id:usize)->bool;
 fn keep_marker(&self,id:usize)->bool;
 fn detach(&mut self,id:usize)->Result<()>;
}
fn collect(tree:&impl Tree,id:usize,keep:&mut BTreeSet<usize>){
 if tree.keep_marker(id)||tree.selection_box(id){keep.insert(id);return}
 if tree.is_node(id){for child in tree.children(id).iter().flatten(){collect(tree,*child,keep)}}
}
fn trim(tree:&mut impl Tree,id:usize,keep:&BTreeSet<usize>)->Result<()>{
 if keep.contains(&id){return Ok(())}
 let ancestor=keep.iter().any(|kept|{let mut p=tree.parent(*kept);while let Some(parent)=p{if parent==id{return true}p=tree.parent(parent)}false});
 if ancestor{if tree.is_node(id){let children=tree.children(id).to_vec();for child in children.into_iter().flatten(){trim(tree,child,keep)?}}}
 else{tree.detach(id)?}Ok(())
}
pub(super) fn remove_hidden(tree:&mut impl Tree,id:usize)->Result<()>{
 if !tree.hidden(id)||tree.parent(id).is_none(){
  if tree.is_node(id){let children=tree.children(id).to_vec();for child in children.into_iter().flatten(){remove_hidden(tree,child)?}}
 }else{let mut keep=BTreeSet::new();collect(tree,id,&mut keep);trim(tree,id,&keep)?}Ok(())
}
#[cfg(test)]mod tests{
 use super::*;
 struct Nodes(Vec<serde_json::Value>);
 impl Tree for Nodes{
  fn parent(&self,i:usize)->Option<usize>{self.0[i]["parent"].as_u64().map(|v|v as usize)}
  fn children(&self,i:usize)->&[Option<usize>]{unreachable!("separate typed fixture adapter below {i}")}
  fn is_node(&self,i:usize)->bool{self.0[i]["kind"]!="shape"}
  fn hidden(&self,i:usize)->bool{self.0[i]["flags"].as_u64().unwrap_or(16)&1!=0}
  fn selection_box(&self,i:usize)->bool{self.0[i]["name"].as_str().unwrap().eq_ignore_ascii_case("SelectionBox")}
  fn keep_marker(&self,i:usize)->bool{self.0[i]["keep"].as_str().is_some_and(|s|["fiducial","sgoKeep","NiOptimizeKeep","NiBoneLOD#"].iter().any(|m|s.contains(m)))}
  fn detach(&mut self,_:usize)->Result<()>{unreachable!()}
 }
 struct Fixture{source:Nodes,children:Vec<Vec<Option<usize>>>,parents:Vec<Option<usize>>}
 impl Tree for Fixture{
  fn parent(&self,i:usize)->Option<usize>{self.parents[i]}
  fn children(&self,i:usize)->&[Option<usize>]{&self.children[i]}
  fn is_node(&self,i:usize)->bool{self.source.is_node(i)}
  fn hidden(&self,i:usize)->bool{self.source.hidden(i)}
  fn selection_box(&self,i:usize)->bool{self.source.selection_box(i)}
  fn keep_marker(&self,i:usize)->bool{self.source.keep_marker(i)}
  fn detach(&mut self,i:usize)->Result<()>{let p=self.parents[i].ok_or("fixture parent absent")?;let slot=self.children[p].iter().position(|c|*c==Some(i)).ok_or("fixture child absent")?;self.children[p][slot]=None;self.parents[i]=None;Ok(())}
 }
 #[test]fn native_hidden_tree_membership_all_pc(){
  let native:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-model-resource-load/native-model-prelude-v2.json")).unwrap();
  for row in native["rows"].as_array().unwrap().iter().filter(|r|r["kind"]=="hidden"){
   let nodes=row["input"].as_array().unwrap().clone();let before=row["output"]["before"].as_array().unwrap();
   let children=before.iter().map(|n|serde_json::from_value(n["children"].clone()).unwrap()).collect();let parents=before.iter().map(|n|n["parent"].as_u64().map(|v|v as usize)).collect();
   let mut fields=Fixture{source:Nodes(nodes),children,parents};remove_hidden(&mut fields,0).unwrap();
   let actual=(0..fields.parents.len()).map(|i|serde_json::json!({"parent":fields.parent(i),"children":fields.children(i)})).collect::<Vec<_>>();
   let expected=row["output"]["after"].as_array().unwrap().iter().map(|n|serde_json::json!({"parent":n["parent"],"children":n["children"]})).collect::<Vec<_>>();assert_eq!(actual,expected,"{}",row["input"]);
  }
 }
}

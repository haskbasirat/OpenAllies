//!86EAB0/86EFB0 propagation over borrowed source owners. Copying a state
//!copies references to properties; it never clones Property allocations.
use std::sync::Arc;
type Result<T>=std::result::Result<T,String>;
pub(super) trait Tree<V:Clone>{
 fn children(&self,node:usize)->&[Option<usize>];
 fn own_count(&self,node:usize)->usize;
 fn own(&self,node:usize,index:usize)->Result<(usize,V)>;
 fn geometry(&self,node:usize)->bool;
 fn retain(&mut self,node:usize,state:Arc<[V;12]>)->Result<()>;
}
pub(super) fn propagate<V:Clone>(tree:&mut impl Tree<V>,root:usize,defaults:[V;12])->Result<()>{
 fn visit<V:Clone>(tree:&mut impl Tree<V>,node:usize,incoming:Arc<[V;12]>)->Result<()>{
  let state=if tree.own_count(node)==0{incoming}else{
   let mut slots=(*incoming).clone();for index in 0..tree.own_count(node){let(kind,owner)=tree.own(node,index)?;let slot=slots.get_mut(kind).ok_or("actual Property enum escapes twelve-state directory")?;*slot=owner;}Arc::new(slots)
  };
  if tree.geometry(node){tree.retain(node,Arc::clone(&state))?;}
  let children=tree.children(node).to_vec();for child in children.into_iter().flatten(){visit(tree,child,Arc::clone(&state))?;}Ok(())
 }
 visit(tree,root,Arc::new(defaults))
}
#[cfg(test)]mod tests{
 use super::*;
 struct Node{children:Vec<Option<usize>>,own:Vec<(usize,String)>,geometry:bool,state:Option<Arc<[String;12]>>}
 struct Graph(Vec<Node>);
 impl Tree<String> for Graph{
  fn children(&self,i:usize)->&[Option<usize>]{&self.0[i].children}
  fn own_count(&self,i:usize)->usize{self.0[i].own.len()}
  fn own(&self,i:usize,j:usize)->Result<(usize,String)>{Ok(self.0[i].own[j].clone())}
  fn geometry(&self,i:usize)->bool{self.0[i].geometry}
  fn retain(&mut self,i:usize,state:Arc<[String;12]>)->Result<()>{self.0[i].state=Some(state);Ok(())}
 }
 #[test]fn actual_initialized_defaults_and_source_overrides_retain_native_aliases(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-property-state.json")).unwrap();
  for row in fixture["rows"].as_array().unwrap(){let mut graph=Graph(vec![Node{children:vec![Some(1),None,Some(2)],own:vec![],geometry:false,state:None},Node{children:vec![],own:vec![],geometry:true,state:None},Node{children:vec![Some(3),Some(4)],own:vec![],geometry:false,state:None},Node{children:vec![],own:vec![],geometry:true,state:None},Node{children:vec![],own:vec![],geometry:true,state:None}]);
   match row["override"].as_str().unwrap(){"none"=>(),"root"=>graph.0[0].own.push((3,"s0".into())),"child"=>graph.0[3].own.push((3,"s0".into())),"both"=>{graph.0[0].own.push((3,"s0".into()));graph.0[3].own.push((3,"s1".into()))},"shared"=>{graph.0[0].own.push((3,"s0".into()));graph.0[3].own.push((3,"s0".into()))},_=>panic!()}
   propagate(&mut graph,0,std::array::from_fn(|i|format!("d{i}"))).unwrap();
   let states=graph.0.iter().map(|node|node.state.as_ref().map(|state|state.as_ref().to_vec())).collect::<Vec<_>>();assert_eq!(serde_json::json!(states),row["states"]);
   let mut aliases=vec![];for a in 0..graph.0.len(){for b in a+1..graph.0.len(){if let(Some(x),Some(y))=(&graph.0[a].state,&graph.0[b].state){if Arc::ptr_eq(x,y){aliases.push([a,b]);}}}}assert_eq!(serde_json::json!(aliases),row["aliases"]);
  }
 }
}

from pathlib import Path
p=Path('crates/aa-mind/tests/cai_arrivals_native.rs');s=p.read_text();a=s.index('#[test]\nfn native_sleep_peek');b=s.index('#[test]\nfn actual_regiment',a)
s=s[:a]+'''struct Drain {list:List,current:std::collections::BTreeMap<u32,(u32,bool)>,trace:Vec<Value>,freed:Vec<u32>,fail:Option<u32>}
impl aa_mind::cai_arrivals::DrainWorld for Drain {
 fn front(&mut self,_:MindId,_:&Arena)->aa_mind::Result<Option<u32>>{Ok(self.list.front())}
 fn resolve(&mut self,g:u32)->aa_mind::Result<Option<u32>>{self.trace.push(json!(["resolve",g]));Ok(self.current.get(&(g&0xffff)).filter(|(v,_)|v>>16==g>>16).map(|_|g))}
 fn pop_front(&mut self,_:MindId,_:&Arena)->aa_mind::Result<()>{let g=self.list.pop_front().unwrap();self.trace.push(json!(["head",u8::from(!self.list.is_empty())]));if self.list.is_empty(){self.trace.push(json!(["tail",0]))}self.trace.push(json!(["free",g]));self.freed.push(g);Ok(())}
 fn member_organization(&mut self,g:u32)->aa_mind::Result<u32>{Ok(u32::from(self.current[&(g&0xffff)].1))}
 fn organization(&mut self,_:MindId,_:&Arena)->aa_mind::Result<u32>{Ok(1)}
 fn member_arrived(&mut self,_:MindId,g:u32,_:&mut Arena)->aa_mind::Result<()>{self.trace.push(json!(["arrived",g]));if self.fail==Some(g){return Err(aa_mind::Error::World("arrived".into()))}Ok(())}
}
#[test]
fn native_sleep_peek_resolve_then_unlink_tail_before_callback(){
 let doc=fixture();let rows=doc["drain_rows"].as_array().unwrap();assert_eq!(rows.len(),30);
 for row in rows{let c=&row["input"];let input=words(&c["values"]);let mut current=std::collections::BTreeMap::new();for(i,&g)in input.iter().enumerate(){if g&0xffff!=0&&i%3!=1{current.insert(g&0xffff,(g,i%3==0));}}let mut w=Drain{list:List::from_saved(input),current,trace:vec![],freed:vec![],fail:c["fail_after"].as_u64().map(|v|v as u32)};let _=aa_mind::cai_arrivals::drain(MindId(1),&mut Arena::default(),&mut w);assert_eq!(json!({"values":w.list,"trace":w.trace,"freed":w.freed}),row["output"],"{c}");}
}
''' +s[b:];p.write_text(s)

//!879D77..879F1E derived material/property extras. The caller owns the same
//!Property/Node allocation and the separate existing-name lookup. No native
//!allocator layout, unwritten alpha padding or renderer readiness is invented.
#[derive(Debug,PartialEq,Eq)]
pub(super) struct Opacity{bytes:[u8;12],known:[bool;12]}
impl Opacity{
 fn byte(&mut self,index:usize,value:u8){self.bytes[index]=value;self.known[index]=true;}
 fn word(&mut self,index:usize,value:u32){for(i,b)in value.to_le_bytes().into_iter().enumerate(){self.byte(index+i,b);}}
 pub(super) fn read(&self,index:usize)->Option<u8>{self.known.get(index).copied().unwrap_or(false).then(||self.bytes[index])}
}
///The native source property's ambient/diffuse/specular/emissive words are
///24..50. Its cached48-byte omi stores diffuse before ambient.
pub(super) fn material(words:[u32;14])->[u32;12]{
 [words[3],words[4],words[5],words[0],words[1],words[2],words[6],words[7],words[8],words[9],words[10],words[11]]
}
///Only actually written bytes are made readable. With no alpha property,
///native code leaves fields1/2/8 and padding unwritten, even on fresh data.
pub(super) fn opacity(material:Option<&[u32;14]>,alpha:Option<(u16,u8)>)->Opacity{
 let mut out=Opacity{bytes:[0;12],known:[false;12]};
 out.byte(0,u8::from(alpha.is_some()));
 if let Some((flags,threshold))=alpha{out.byte(1,((flags>>9)&1)as u8);out.byte(2,(flags&1)as u8);out.byte(8,threshold);}
 out.word(4,material.map_or(0x3f800000,|m|m[13]));out
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn material_and_opacity_written_fields_match_original_subblock_all_pc(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-property-extras.json")).unwrap();
  for row in fixture["rows"].as_array().unwrap(){let input=&row["input"];let words=serde_json::from_value::<[u32;14]>(row["source_words"].clone()).unwrap();
   if input["material"].as_bool().unwrap()&&!input["old_omi"].as_bool().unwrap(){let actual=material(words).into_iter().flat_map(u32::to_le_bytes).collect::<Vec<_>>();let expected=row["material_omi"]["bytes"].as_str().unwrap();assert_eq!(actual.iter().map(|b|format!("{b:02x}")).collect::<String>(),expected);}
   if !input["old_opi"].as_bool().unwrap(){let alpha=input["alpha"].as_array().map(|v|(v[0].as_u64().unwrap()as u16,v[1].as_u64().unwrap()as u8));let data=opacity(input["material"].as_bool().unwrap().then_some(&words),alpha);let bytes=(0..12).map(|i|data.read(i).unwrap_or(0xa5)).collect::<Vec<_>>();let expected=row["node_opi"]["bytes"].as_str().unwrap();assert_eq!(bytes.iter().map(|b|format!("{b:02x}")).collect::<String>(),expected);}
  }
 }
}

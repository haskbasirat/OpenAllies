//! Bounded native fresh SVars node field-table projection. Authored fields are
//! inserted into the real five-bucket/prepend/growing order; values are lazy.
//! Inherited or prefilled DataNode hash maps require a separate producer.
use super::ResolvedNode;
use crate::{actors::RawField,native::{Error,Result,ScalarType,TypedScalar,TypedValue},resources::Provenance};
use aa_numeric::Precision;
use std::collections::BTreeSet;

/// A read-only token into the same source definition. Obtaining ordered tokens
/// does not parse their values: Settings may skip an existing flags2 variable.
pub struct SVarField<'a>{node:ResolvedNode<'a>,field:&'a RawField}
impl SVarField<'_>{
 pub fn name(&self)->&str{&self.field.name}
 pub fn declared_type(&self)->Option<&str>{self.field.declared_type.as_deref()}
 pub fn source(&self)->&Provenance{&self.field.source}
 pub fn raw(&self)->Result<&str>{
  let bytes=self.field.value.as_deref().ok_or_else(||Error::Unsupported(format!("SVar {} has no assigned value",self.name())))?;
  if !bytes.is_ascii(){return Err(Error::Unsupported(format!("SVar {} has non-ASCII literal bytes",self.name())))}
  std::str::from_utf8(bytes).map_err(|_|Error::Unsupported(format!("SVar {} has invalid literal bytes",self.name())))
 }
 /// DataField virtual48, not the destination SVar's kind. Inferred source
 /// Bool and ordinary decimal Float tokens are explicit native parser scope.
 /// Unknown inference never silently becomes a String.
 pub fn source_kind(&self)->Result<u32>{
  if let Some(kind)=self.declared_type(){return match kind.to_ascii_lowercase().as_str(){
   "int"=>Ok(2),"float"=>Ok(3),"fixed"=>Ok(4),"bool"|"flag"=>Ok(5),"string"=>Ok(6),"point3"=>Ok(8),"localized"=>Ok(9),
   _=>Err(Error::Unsupported(format!("SVar {} native declared type {kind} unbound",self.name()))),
  }}
  let raw=self.raw()?;
  if raw.eq_ignore_ascii_case("true")||raw.eq_ignore_ascii_case("false"){return Ok(5)}
  let digits=raw.strip_prefix(['+','-']).unwrap_or(raw);
  if let Some((whole,fraction))=digits.split_once('.')
   &&(!whole.is_empty()||!fraction.is_empty())
   &&whole.bytes().all(|b|b.is_ascii_digit())&&fraction.bytes().all(|b|b.is_ascii_digit()) {return Ok(3)}
  Err(Error::Unsupported(format!("SVar {} inferred native field type is unbound",self.name())))
 }
 /// Called only at the native reached getter. Earlier current-store effects
 /// remain owned by the caller if this or a subsequent token fails.
 pub fn value(&self,pc:Precision)->Result<TypedValue>{
  let kind=self.source_kind()?;
  if self.declared_type().is_some(){return self.node.field(self.name(),pc)?.ok_or_else(||Error::Unsupported(format!("SVar {} source field vanished",self.name())))}
  let source=self.source().clone();let declaration_source=source.clone();let raw=self.raw()?;
  match kind{
   5=>Ok(TypedValue::Boolean{value:raw.eq_ignore_ascii_case("true"),source,declaration_source}),
   3=>Ok(TypedValue::Scalar(TypedScalar{bits:crate::native::scalar_literal(raw,pc)?,field_type:ScalarType::Float,source,declaration_source})),
   _=>Err(Error::Unsupported(format!("SVar {} inferred value getter is unbound",self.name()))),
  }
 }
}
fn hash(name:&str)->u32{name.bytes().fold(0u32,|h,b|h.wrapping_mul(33).wrapping_add(u32::from(b.to_ascii_uppercase())))}
fn indices(names:&[&str])->Result<Vec<usize>>{
 let mut seen=BTreeSet::new();let mut buckets=vec![Vec::<usize>::new();5];
 for(index,name)in names.iter().enumerate(){
  if name.is_empty()||!name.is_ascii()||name.bytes().any(|b|!b.is_ascii_alphanumeric()&&b!=b'_'){
   return Err(Error::Unsupported("fresh SVar field hash requires an ASCII identifier".into()))
  }
  if !seen.insert(name.to_ascii_lowercase()){return Err(Error::Unsupported(format!("fresh SVar node repeats field {name}")))}
  if index>=buckets.len().checked_mul(2).ok_or_else(||Error::Unsupported("SVar field table size overflow".into()))?{
   let mut size=buckets.len().checked_mul(2).and_then(|v|v.checked_add(1)).ok_or_else(||Error::Unsupported("SVar field table size overflow".into()))?;
   while size%3==0||size%5==0{size=size.checked_add(2).ok_or_else(||Error::Unsupported("SVar field table growth overflow".into()))?;}
   let mut next=vec![Vec::new();size];
   for chain in buckets{for prior in chain{next[hash(names[prior])as usize%size].insert(0,prior)}}
   buckets=next;
  }
  let slot=hash(name)as usize%buckets.len();buckets[slot].insert(0,index);
 }
 Ok(buckets.into_iter().flatten().collect())
}
impl<'a> ResolvedNode<'a>{
 /// Native4B3C10 DataNode+40 traversal for a fresh, untemplated SVars node.
 /// Source and field identities remain borrowed; values are not eager.
 pub fn native_svar_fields(&self)->Result<Vec<SVarField<'a>>>{
  if !self.catalog.resolver.source_hashes_valid{return Err(Error::Unsupported("SVar field source hashes are not verified".into()))}
  let [node]=self.layers.as_slice()else{return Err(Error::Unsupported("SVar field table inheritance/prefilled state is unbound".into()))};
  if !node.name.eq_ignore_ascii_case("SVars")||node.template||node.optional||node.list||node.base.is_some()||node.bound.is_some()||node.problem.is_some()||!node.children.is_empty(){
   return Err(Error::Unsupported("native fresh SVar field table requires one ordinary untemplated childless SVars node".into()))
  }
  if node.fields.iter().any(|f|f.unsupported_declaration){return Err(Error::Unsupported("SVar field declaration syntax is unbound".into()))}
  let order=indices(&node.fields.iter().map(|f|f.name.as_str()).collect::<Vec<_>>())?;
  Ok(order.into_iter().map(|index|SVarField{node:self.clone(),field:&node.fields[index]}).collect())
 }
}
#[cfg(test)]
#[path="native_field_order_tests.rs"]
mod tests;

/// Pure metadata ordering for an explicitly fresh five-bucket native field
/// table. This is not a parsed-node, source-owner or successful-load witness.
/// Actual document users must validate their own source bytes and field rows.
pub fn native_fresh_field_order(names:&[&str])->Result<Vec<usize>>{indices(names)}

//!5A7230 KKC_NiSeqResource::LoadSequence and 5A5650
//!KKC_NiSeqResource::ComputeLatestTimeController over the selected KF bytes,
//!through the aa-formats port of NiStream::Load (proven on every shipped KF:
//!research/phase8-nif native-kf-corpus.tsv / native-kf-fields.tsv).
//!
//!5A7230: the shared stream (B36510+0x74) loads the bytes; a failed load,
//!no top-level object or a stream version below 4.1.0.3 (880060 of
//!A45968) reports "Corrupt sequence: %s" (4252A0) and returns NULL. The
//!first top-level object cast to NiControllerSequence (711540 B4419C) is the
//!result; an NiSequence (B441AC) would be converted by 8847B0 (with the
//!0x74E "needs to be re-exported" report), which no shipped KF uses and is
//!an explicit error here. 880990 then releases the stream's objects.
//!
//!5A5650: the latest stop time (+0x1C) of the +0x2C controllers, from 0.0,
//!kept only when strictly greater; the first APP_INIT controller (flags
//!bit 0) is reported once (0x729), and a count of 0 or a result not above
//!0.0 is reported (0x733).
use aa_formats::nif::{self,NifFile};
use aa_numeric::Binary;
use std::cmp::Ordering;
type Result<T>=std::result::Result<T,String>;

///The decoded sequence: the native object graph and the root's index.
#[derive(Debug)]
pub(crate) struct Sequence{pub(crate) nif:NifFile,pub(crate) root:usize}

#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) enum Report{
 ///4252A0 "Corrupt sequence: %s" (the file name).
 Corrupt,
 ///404ED0 line 0x729 "Sequence %hs time controller not APP_TIME".
 NotAppTime(String),
 ///404ED0 line 0x733 "Sequence %hs total length is 0".
 ZeroLength(String),
}
impl Report{
 pub(crate) fn line(&self)->Option<u32>{match self{Self::Corrupt=>None,Self::NotAppTime(_)=>Some(0x729),Self::ZeroLength(_)=>Some(0x733)}}
 pub(crate) fn text(&self,filename:&str)->String{match self{Self::Corrupt=>format!("Corrupt sequence: {filename}"),Self::NotAppTime(n)=>format!("Sequence {n} time controller not APP_TIME"),Self::ZeroLength(n)=>format!("Sequence {n} total length is 0")}}
}

fn name(nif:&NifFile,i:usize)->String{nif.objects[i].fields.string("name").flatten().map(|b|b.split(|&c|c==0).next().unwrap_or(&[]).iter().map(|&c|c as char).collect()).unwrap_or_default()}

///5A7230; None is the NULL result after its Corrupt report.
pub(crate) fn load_sequence(raw:&[u8],report:&mut dyn FnMut(Report)->Result<()>)->Result<Option<Sequence>>{
 //LoadStream fails (returns false) on these; truncation, bad links and
 //unported classes are not native failures.
 let nif=match nif::load(raw){Ok(nif)=>nif,
  Err(nif::NifError::NotNif|nif::NifError::VersionTooOld(_)|nif::NifError::UnknownVersion(_)|nif::NifError::UserVersionTooOld(_)|nif::NifError::UnknownUserVersion(_)|nif::NifError::NoCreateFunction(_))=>{report(Report::Corrupt)?;return Ok(None)}
  Err(e)=>return Err(format!("5A7230 selected KF: {e}"))};
 if !nif.requires_native_post_process.is_empty(){return Err("5A7230 KF with old particle objects requires the native 8E93B0 post-load conversion".into())}
 let Some(first)=nif.top_level.first().copied()else{report(Report::Corrupt)?;return Ok(None)};
 if nif.version<0x0401_0003{report(Report::Corrupt)?;return Ok(None)}
 match first.map(|i|nif.objects[i].class){
  Some("NiControllerSequence")=>Ok(Some(Sequence{root:first.expect("root"),nif})),
  Some("NiSequence")=>Err("5A72F6 8847B0 old NiSequence conversion is not ported".into()),
  _=>{report(Report::Corrupt)?;Ok(None)}
 }
}

///5A5650 over the decoded root; the length as binary32 bits.
pub(crate) fn latest_time_controller(s:&Sequence,report:&mut dyn FnMut(Report)->Result<()>)->Result<u32>{
 let nif=&s.nif;let controllers=nif.sequence_controllers(s.root).map_err(|e|e.to_string())?;
 let f=|w:u32|Binary::from_f32_bits(w).map_err(|e|e.to_string());
 let mut latest=0u32;let mut reported=false;
 for c in &controllers{
  let c=c.ok_or("5A5683 reads the flags of a NULL sequence controller")?;let o=&nif.objects[c].fields;
  if !reported&&o.u16("flags").ok_or("89F070 flags absent")?&1!=0{report(Report::NotAppTime(name(nif,s.root)))?;reported=true;}
  let stop=o.f32("stop").ok_or("89F070 stop absent")?;
  if f(stop)?.compare(&f(latest)?)==Ordering::Greater{latest=stop}
 }
 if controllers.is_empty()||f(latest)?.compare(&Binary::from_u32(0))!=Ordering::Greater{report(Report::ZeroLength(name(nif,s.root)))?;}
 Ok(latest)
}

///The 5A7230 decode service of 5A8190: the root and its 5A5650 length,
///reports logged as the native 404ED0/4252A0 messages.
pub(crate) fn decode_service(filename:&str,raw:&[u8])->Result<Option<(aa_runtime::sequence_resource_load::SequencePayload,u32)>>{
 let mut log=|r:Report|->Result<()>{match r.line(){Some(line)=>log::warn!("native404ED0 level=1 KKC_NiSeqResource line {line:#x}: {}",r.text(filename)),None=>log::error!("native4252A0 {}",r.text(filename))}Ok(())};
 let Some(sequence)=load_sequence(raw,&mut log)?else{return Ok(None)};
 let length=latest_time_controller(&sequence,&mut log)?;
 Ok(Some((aa_runtime::sequence_resource_load::SequencePayload(std::sync::Arc::new(sequence)),length)))
}

#[cfg(test)]mod tests{
 use super::*;
 ///Every shipped KF against the original 5A5650 run on its natively loaded
 ///root (research/phase8-nif/native_kf_length.py): root class, length bits
 ///and the reported 404ED0 lines.
 #[test]fn every_shipped_kf_length_matches_native(){
  let here=std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
  let root=std::env::var_os("AA_GAME_ROOT").map(std::path::PathBuf::from).unwrap_or_else(||here.join("../../.."));
  let Ok(mut archive)=aa_formats::RwdArchive::open(root.join("Data.rwd"))else{eprintln!("skipping: Data.rwd absent");return};
  let tsv=std::fs::read_to_string(here.join("../../research/phase8-nif/native-kf-length.tsv")).expect("native KF length fixture");
  let native:std::collections::BTreeMap<&str,Vec<&str>>=tsv.lines().skip(1).map(|l|{let c:Vec<&str>=l.split('\t').collect();(c[0],c)}).collect();
  let members:Vec<(usize,String)>=archive.entries().iter().enumerate().filter(|(_,e)|e.path().to_ascii_lowercase().ends_with(".kf")).map(|(i,e)|(i,e.path().to_string())).collect();
  assert_eq!(members.len(),native.len());
  let mut differ=Vec::new();
  for(i,path)in members{
   let mut data=Vec::new();archive.copy_entry(i,&mut data).unwrap();
   let mut lines=Vec::new();let s=load_sequence(&data,&mut|r|{lines.push(r.line());Ok(())}).unwrap().unwrap_or_else(||panic!("{path}"));
   let length=latest_time_controller(&s,&mut|r|{lines.push(r.line());Ok(())}).unwrap();
   let reports=lines.iter().map(|l|format!("{:x}",l.unwrap_or(0))).collect::<Vec<_>>().join(",");
   let row=&native[path.as_str()];
   if row[1]!=s.nif.objects[s.root].class||row[2]!=format!("{length:08x}")||row.get(3).copied().unwrap_or("")!=reports{differ.push(format!("{path}: {length:08x} [{reports}] vs {:?}",row));}
  }
  assert!(differ.is_empty(),"{}",differ.join("\n"));
 }
}

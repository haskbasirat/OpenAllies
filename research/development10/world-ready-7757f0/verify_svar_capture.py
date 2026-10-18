"""Read-only replay of the independent54-file/FP/CV registration audit.

Does not execute AA.exe or read the user's profile. Existing audit/reference
bytes are immutable inputs, and no output fixture is written by this script.
"""
from pathlib import Path
import hashlib,json,collections,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
def load(p):return json.loads(p.read_text(encoding='utf-8-sig'))
def sha(p):
 h=hashlib.sha256()
 with p.open('rb')as f:
  while b:=f.read(8*1024*1024):h.update(b)
 return h.hexdigest()
def pin(p,digest):assert sha(p)==digest,(str(p),sha(p))
def main():
 capture_path=ROOT/'research/development10/native-svar-inventory-capture-02/capture.json'
 old_path=ROOT/'research/development10/native-source-capture-03/capture.json'
 manifest_path=old_path.parent/'runtime-files.json'
 audit_path=HERE/'svar-capture-support-verification.json';ref_path=HERE/'svar-registration-reference.json'
 pin(audit_path,'433be4e3b0b1b16490a80f14a482355abd0af659d01b70f51ab10c1a50e51810')
 pin(ref_path,'835544ff092751412e6df08cee9ce3a56197677e382430820bda207a505b3800')
 audit=load(audit_path);ref=load(ref_path)
 for path,key in [(capture_path,'capture_sha256'),(old_path,'reference_source03_sha256'),(manifest_path,'reference_manifest_sha256')]:pin(path,audit[key])
 capture=load(capture_path);old=load(old_path);manifest=load(manifest_path)
 assert capture['original_error']is None and capture['completed_source_return']
 assert capture['folder_redirection_observed']and capture['fp_state_never_written_by_debugger']
 assert capture['original_archives_preserved']and capture['cleanup']['errors']==[]
 assert capture['cleanup']['exit_process_event_observed']and capture['cleanup']['wait_result']==0
 assert len(manifest['records'])==len(audit['records'])==54
 source=Path(manifest['source']);runtime=capture_path.parent/'runtime'
 for expected,observed in zip(manifest['records'],audit['records']):
  assert expected['path']==observed['path']
  a=source/expected['path'];b=runtime/expected['path'];digest=expected['source_sha256']
  assert a.stat().st_size==b.stat().st_size==expected['bytes']==observed['bytes']
  assert sha(a)==sha(b)==digest==observed['sha256']
  assert not a.samefile(b)and observed['copies_independent']
  readonly=bool(getattr(b.stat(),'st_file_attributes',0)&1)
  assert readonly==observed['copy_readonly']
  if a.suffix.lower()=='.rwd':assert readonly
 assert len(capture['captures'])==len(old['captures'])==4
 for current,previous in zip(capture['captures'],old['captures']):
  assert current['label']==previous['label']
  for key in ['legacy_x87_control','legacy_x87_status','fxsave_x87_control','fxsave_x87_status','x87_control','x87_status','mxcsr','crt_sse2_global','globals','manager_means']:
   assert current[key]==previous[key],(current['label'],key)
  for key in ['settings','ranged_cached']:
   assert (key in current)==(key in previous)
   if key in current:assert {k:v['bits']for k,v in current[key].items()}=={k:v['bits']for k,v in previous[key].items()}
  for key in ['actors','csv_length']:
   assert (key in current)==(key in previous)
   if key in current:assert current[key]==previous[key]
 assert len(capture['captures'][-1]['actors'])==1028
 native=capture['captures'][-1]['registered_svars'];entries=native['entries']
 assert native['manager']==ref['manager']==0xb39aa8
 assert native['count']==ref['count']==len(entries)==231
 assert ref['source_control']==capture['captures'][-1]['x87_control']==0x027f
 assert {str(k):v for k,v in collections.Counter(e['kind']for e in entries).items()}==ref['kind_counts']
 projected=[];strings=[]
 for ordinal,e in enumerate(entries):
  assert ordinal==e['ordinal']==e['word20']and'\0'not in e['name']
  h=struct.unpack('<4I',bytes.fromhex(e['name_header']))
  assert h[0]!=0 and h[1]>=1 and h[1]<=h[2]and h[3]>=1
  projected.append({k:e[k]for k in ['ordinal','name','kind','word20']})
  if e['kind']==4:
   h=struct.unpack('<4I',bytes.fromhex(e['value_string_header']))
   assert h[1]==e['value_string_length']<=h[2]and h[3]>=1
   strings.append({k:e[k]for k in ['ordinal','name','value_string_header','value_string_length']})
 assert projected==ref['entries']and strings==ref['string_storage']
 assert audit['all1028_actor_triples_flags_and_means_match']and audit['all_four_numeric_controls_match']
 assert ref['support_verification_sha256']==sha(audit_path)
 print(json.dumps({'files':54,'controls':4,'actors':1028,'registrations':231,'passed':True}))
if __name__=='__main__':main()

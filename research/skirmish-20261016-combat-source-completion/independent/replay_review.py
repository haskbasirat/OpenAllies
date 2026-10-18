"""Independent read-only receipt checks and unchanged native whole-bank replay."""
from pathlib import Path
import hashlib,json,re,struct,sys,time
HERE=Path(__file__).resolve().parent;SOURCE=HERE.parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(SOURCE));import native_source_completion as native
sha=lambda p:hashlib.sha256(Path(p).read_bytes()).hexdigest()
def text(buffer,s):
 raw=s.encode();buffer.extend(struct.pack('<Q',len(raw)));buffer.extend(raw)
def result_hash(means,rows):
 b=bytearray(b'AA native combat cache v1\0');b.extend(struct.pack('<Q',len(rows)))
 for x in means:b.extend(struct.pack('<I',x))
 for ident,flags,fields in rows:
  text(b,ident);b.extend(struct.pack('<I',flags&0x245e9));b.extend(struct.pack('<III',*fields))
 return hashlib.sha256(b).hexdigest()
def main():
 files=[SOURCE/'native_source_completion.py',SOURCE/'full-source-inputs-v3.json',SOURCE/'native-source-completion-v3.json',ROOT/'crates/aa-runtime/src/combat_source_receipt.rs']
 before={str(p):sha(p)for p in files};fixture=json.loads(files[2].read_text());inputs=json.loads(files[1].read_text())
 assert fixture['producer_sha256']==sha(files[0]) and fixture['input_sha256']==sha(files[1])
 code=files[3].read_text();constants=dict(re.findall(r'pub const (\w+): &str\s*=\s*"([0-9a-f]+)";',code))
 capture_dir=ROOT/'research/development10/native-source-capture-03';capture_path=capture_dir/'capture.json'
 capture=json.loads(capture_path.read_text());manifest=json.loads((capture_dir/'runtime-files.json').read_text())
 game=ROOT.parent;runtime=game/'reconstruction/native-game/runtime-expanded-v14'
 checked={}
 for constant,path in [('CAPTURE_SHA256',capture_path),('FILE_MANIFEST_SHA256',capture_dir/'runtime-files.json'),('EXE_SHA256',runtime/'AA.exe'),('RENDERER_SHA256',runtime/'D3D9.dll'),('RENDERER_CONFIG_SHA256',runtime/'dgVoodoo.conf')]:
  assert sha(path)==constants[constant],constant;checked[constant]=constants[constant]
 assert len(manifest['records'])==54
 for row in manifest['records']:
  assert row['source_sha256']==row['copy_sha256_after_capture']==sha(runtime/row['path'])==sha(capture_dir/'runtime'/row['path'])
  assert row['same_file'] is False
  if row['path'].lower().endswith('.rwd'):assert row['copied_readonly_attribute']
 assert capture['completed_source_return']and capture['fp_state_never_written_by_debugger']and capture['folder_redirection_observed']
 assert capture['get_context_flags']==0x1003f and capture['set_context_flags']==0x10001
 assert sha(ROOT/'research/development10/capture_native_source.py')==capture['script_sha256']
 assert capture['original_archives_preserved'] and not capture['cleanup']['errors'] and capture['cleanup']['wait_result']==0 and capture['cleanup']['exit_process_event_observed']
 for row in capture['captures']:
  assert row['legacy_fxsave_match']and row['legacy_x87_control']==row['fxsave_x87_control']==row['x87_control']==0x27f
  assert row['mxcsr']==0x1fbd and row['crt_sse2_global']==1
  assert bytes.fromhex(row['stream_object_bytes'])[4:8]==b'\0'*4
 entry=next(c for c in capture['captures']if c['label']=='source_entry')
 assert entry['x87_status']==0x120 and entry['globals']=={'assertion_guard':0,'csv_guard':0,'ranged_guard':0}
 b=bytearray(b'AA source identities v1\0');sources=inputs['source_identities'];assert len(sources)==1647;b.extend(struct.pack('<Q',len(sources)))
 for row in sources:
  text(b,row['path']);text(b,row['sha256'])
  if isinstance(row['origin'],dict):b.append(0);b.extend(struct.pack('<Q',row['origin']['OriginalArchive']['member_index']))
  else:assert row['origin']=='V14Loose';b.append(1)
 assert hashlib.sha256(b).hexdigest()==constants['SOURCES_SHA256']
 returned=next(c for c in capture['captures']if c['label']=='source_return');assert len(returned['actors'])==1028
 means=[returned['manager_means'][f'{off:x}']for off in range(0x74,0x94,4)]
 windowsrows=[(r['ids'],r['flags_174'],[r['fields'][f'{off:x}']for off in [0x1e4,0x30c,0x36c]])for r in returned['actors']]
 assert result_hash(means,windowsrows)==constants['COMBAT_RESULT_SHA256']
 masks=[row['input']['Ok']['flags']for row in inputs['rows']if row['precision']=='Double53']
 assert masks==[row['flags_174']for row in returned['actors']]
 allcases=[]
 for expected in fixture['cases']:
  image=native.h.original if expected['image']=='original'else native.h.modified
  start=time.monotonic();m=native.Machine(expected['precision'],image,inputs)
  m.call(0x4bbf90,ecx=m.manager,count=20_000_000);m.call(0x4bb590,ecx=m.manager,count=40_000_000);m.verify_v14();assert not m.null_accesses
  got={'image':expected['image'],'precision':expected['precision'],'means':[m.r(m.manager+off)for off in range(0x74,0x94,4)],
   'values':[{'id':ident,'value_1e4':m.r(d+0x1e4),'children_30c':m.r(d+0x30c),'weighted_36c':m.r(d+0x36c)}for ident,d in m.actors.items()],
   'events':m.events,'final_assert_guard':m.rb(0xb3b4f8),'final_lookup_guard':m.r(0xb3a66c),'final_csv_guard':m.r(0xb3b4f4)}
  assert got==expected,(expected['image'],expected['precision'])
  assert len(got['values'])==1028 and len(got['events'])==5333
  if expected['precision']=='Double53':
   assert got['means']==means
   for value,(ident,flags,words)in zip(got['values'],windowsrows):assert value['id']==ident and[value['value_1e4'],value['children_30c'],value['weighted_36c']]==words
  # The declared allocators were intercepted; no other native allocator body
  # can be misreported as a successful arithmetic/native fault in this replay.
  assert not any(a in m.visited for a in [0x6f7700,0x6f7650,0x96e19f,0x96e173,0x96e0f8,0x96d331])
  report={'image':expected['image'],'precision':expected['precision'],'values':1028,'events':5333,'seconds':round(time.monotonic()-start,3)};allcases.append(report);print(json.dumps(report),flush=True)
 for pin in fixture['instructions']:
  a=int(pin['va'],16);raw=bytes.fromhex(pin['bytes']);assert native.h.original.range(a,len(raw))==native.h.modified.range(a,len(raw))==raw
 assert before=={str(p):sha(p)for p in files}
 out={'passed':True,'sealed_files':checked,'source_identities':1647,'independent_source_encoding_matches':True,'independent_windows_result_encoding_matches':True,'copied_runtime_files_checked':54,'cases':allcases,'instruction_pins':len(fixture['instructions']),'fixture_and_producer_preserved':True,'file_sha256':before,'scope':'Read-only peer whole-bank arithmetic/receipt replay; no original process launched or full source/world/battle completion claim.'}
 out['all1028_full_component_masks_match_windows']=True
 (HERE/'native-receipt-review-v3.json').write_text(json.dumps(out,indent=2)+'\n');print('PASS',flush=True)
if __name__=='__main__':main()

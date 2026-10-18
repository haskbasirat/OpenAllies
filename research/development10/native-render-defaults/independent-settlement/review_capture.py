"""Read-only review of frozen supplied Windows captures; launches no debuggee."""
from pathlib import Path
import json,hashlib,os,struct,stat,sys
HERE=Path(__file__).resolve().parent;BASE=HERE.parent;ROOT=BASE.parents[2];GAME=ROOT.parent
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as native
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_AC_WRITE
from capstone.x86 import X86_OP_MEM
md=Cs(CS_ARCH_X86,CS_MODE_32);md.detail=True
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
script=BASE/'capture_defaults.py';script_hash=H(script)
native_root=GAME/'reconstruction/native-game/runtime-expanded-v14'
rows=[]
for name in ['capture-01','capture-02']:
 folder=BASE/name;p=folder/'capture.json';doc=json.loads(p.read_text())
 assert doc['script_sha256']==script_hash
 assert doc['original_error']is None and doc['completed_default_init_prefix']and doc['completed_twelve_properties']and doc['completed_default_prefix']
 assert doc['cleanup']['wait_result']==0 and doc['cleanup']['exit_code']==0 and doc['cleanup']['exit_process_event_observed']and not doc['cleanup']['errors']
 assert doc['get_context_flags']==0x1003f and doc['set_context_flags']==0x10001 and doc['debug_register_set_context_flags']==0x10010 and doc['probe_integer_control_set_context_flags']==0x10003
 assert doc['seed_addresses']==[0xb43e68,0xb43e6c,0xb43e70,0xb45f54,0xb323c0]
 assert doc['watched_addresses']==[doc['seed_addresses'][i]for i in doc['watched_word_indices']]
 for arm in doc['watch_arms']:
  assert arm['addresses']==doc['watched_addresses'];assert arm['observed_dr6']&15==0
  assert arm['requested_dr7']&0xffff00ff==arm['observed_dr7']&0xffff00ff
  for i in range(4):assert (arm['observed_dr7']>>(16+4*i))&15==13 and(arm['observed_dr7']>>(2*i))&3==1
 probes=doc['isolated_post_cutoff_watch_calibration_probes'];assert len(probes)==4
 for i,probe in enumerate(probes):
  assert probe['index']==i and probe['address']==doc['watched_addresses'][i]and probe['dr6']&15==1<<i
  assert probe['observed_bits']==probe['value']and probe['eip']==0x76fae7
 assert native.V.range(0x76fae1,6)==bytes.fromhex('899ef8000000')
 snapshots=doc['global_captures'];start=next(r for r in snapshots if r['label']=='create_process_before_program_execution')
 entry=next(r for r in snapshots if r['label']=='defaults_init_entry');end=next(r for r in snapshots if r['label']=='defaults_properties_completed')
 cutoff=next(r for r in snapshots if r['label']=='sealed_real_observation_cutoff')
 assert entry['eip']==0x8c3ed0 and end['eip']==0x8c3f20 and cutoff['eip']==0x8c3f20
 assert start['seed_words']==entry['seed_words']==[0,0,0,0,1]
 assert end['seed_words']==cutoff['seed_words']==doc['observation_cutoff']['seed_words']==[0,0,0,1,2]
 assert all(r['seed_words'][:3]==[0,0,0]for r in snapshots)
 assert all(p['pointer']==0 and p['bytes']is None for p in entry['properties'])
 assert len(end['properties'])==12 and all(p['pointer']and len(bytes.fromhex(p['bytes']))>=8 and struct.unpack_from('<I',bytes.fromhex(p['bytes']),4)[0]==1 for p in end['properties'])
 assert doc['observation_cutoff']['write_count']==len(doc['observed_native_watched_writes'])
 native_writes=[]
 for write in doc['observed_native_watched_writes']:
  assert not set(write['watched_addresses'])&{0xb43e68,0xb43e6c,0xb43e70}
  #Data-breakpoint EIP is after the instruction. Decode a supported concrete
  #writer ending there, rather than falsely calling that EIP a writer pin.
  eip=write['eip'];candidates=[]
  for size in range(1,16):
   try:raw=native.V.range(eip-size,size)
   except ValueError:continue
   instructions=list(md.disasm(raw,eip-size))
   if len(instructions)==1 and instructions[0].size==size:
    ins=instructions[0]
    if any(o.type==X86_OP_MEM and o.access&CS_AC_WRITE and o.mem.disp in write['watched_addresses']for o in ins.operands):candidates.append(dict(address=hex(ins.address),bytes=raw.hex(),instruction=f'{ins.mnemonic} {ins.op_str}'))
  assert candidates,(eip,write['watched_addresses'])
  native_writes.append(dict(observed_after_eip=hex(eip),watched=[hex(a)for a in write['watched_addresses']],candidates=candidates))
 controls=[]
 for sample in [entry,end,cutoff]:
  raw=bytes.fromhex(sample['raw_context']);assert len(raw)==716
  legacy_cw=struct.unpack_from('<I',raw,28)[0]&0xffff;legacy_sw=struct.unpack_from('<I',raw,32)[0]&0xffff
  fx_cw,fx_sw=struct.unpack_from('<HH',raw,204);assert legacy_cw==fx_cw and legacy_sw==fx_sw
  controls.append(dict(label=sample['label'],legacy_fxsave_cw=hex(legacy_cw),legacy_fxsave_sw=hex(legacy_sw),mxcsr=hex(struct.unpack_from('<I',raw,228)[0])))
 for name_,digest in doc['runtime_pins'].items():assert H(folder/'runtime'/name_)==digest
 source_files=[]
 for original in native_root.rglob('*'):
  if not original.is_file():continue
  rel=original.relative_to(native_root)
  if any(part.lower()in['logs','userdata','saves','saved games','profiles']for part in rel.parts):continue
  copied=folder/'runtime'/rel
  assert copied.is_file()and H(original)==H(copied),(original,copied)
  if original.suffix.lower()=='.rwd':assert not os.path.samefile(original,copied)and not copied.stat().st_mode&stat.S_IWRITE
  source_files.append(dict(path=rel.as_posix(),sha256=H(original)))
 assert doc['original_archives_preserved']and all(H(Path(path))==digest for path,digest in doc['original_archives_sha256'].items())
 #The earlier shared-default cutoff can occur before folder paths are used;
 #these traces do show the helper's redirects that actually occurred.
 redirects=[e for e in doc['events']if'folder_redirection_csidl'in e]
 for redir in redirects:assert redir['verified']and folder.resolve()in Path(redir['destination']).resolve().parents
 rows.append(dict(capture=p.relative_to(ROOT).as_posix(),capture_sha256=H(p),pid=doc['owned_process']['pid'],watch_threads=len(doc['watch_arms']),native_writes=native_writes,
  property_count=12,property_refcounts=[1]*12,controls=controls,post_cutoff_calibrations=4,source_files=source_files,
  occurred_folder_redirects=[r['folder_redirection_csidl']for r in redirects],no_fog_write_through_cutoff=True))
out=dict(schema=1,scope='Independent read-only supplied-capture method+file equality review through8C3F20 only. No native rerun, later startup immutability, arbitrary live client context or SharedSet production acceptance yet.',
 script_sha256=script_hash,reviewer_executed_debuggees=0,rows=rows,
 static_method=['DEBUG_ONLY_THIS_PROCESS creation event precedes program resume; every observed CreateThread arms and readbacks fourlocal WRITE4 watches.','CONTROL/debug/integer context writes exclude floating/SSE groups; calibration occurs only after sealed observation cutoff and otherthreads are suspended.','Bounded cleanup uses owned duplicateprocess handle, drains pending debug events and validates exit before closinghandles.','Native process-only patches and same-value calibration do not modify original executable/archives.'])
data=(json.dumps(out,indent=2)+'\n').encode();target=HERE/'capture-method-review.json'
if '--write'in sys.argv:target.write_bytes(data)
else:assert target.read_bytes()==data
print(json.dumps(dict(captures=len(rows),source_files=[len(r['source_files'])for r in rows],sha256=hashlib.sha256(data).hexdigest())))

"""Seal completed real Windows430290 constructor seed captures, read-only by default.
No process launches, debugger actions, source edits or BSS-default inference.
Both traces reached a genuine Custom skirmish launch; probes are post-cutoff.
"""
from pathlib import Path
import hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
def sha(b):return hashlib.sha256(b).hexdigest()
def main():
 addresses=[0xb43cf8,0xb43cfc,0xb43d00,0xb43dbc,0xb43dc0];covered=set();captures=[]
 for name,script in [('capture-02','capture_grid_startup-v1-frozen.py'),('capture-03','capture_grid_startup.py')]:
  folder=HERE/name;raw=(folder/'capture.json').read_bytes();j=json.loads(raw)
  assert j['completed_grid_constructor'] and j['completed_source_return'] and j['completed_session_constructor']
  assert j['original_error']is None and j['original_archives_preserved'] and j['folder_redirection_observed']
  assert j['cleanup']['wait_result']==0 and j['cleanup']['exit_code']==0 and j['cleanup']['exit_process_event_observed'] and not j['cleanup']['errors']
  assert j['source_exe_sha256']==j['runtime_pins']['AA.exe']==L.pe.ACCEPTED_SHA
  assert sha((folder/'runtime/AA.exe').read_bytes())==L.pe.ACCEPTED_SHA
  assert sha((HERE/script).read_bytes())==j['script_sha256']
  assert j['temporary_process_only_breakpoints'] and j['fp_state_never_written_by_debugger']
  assert j['debug_register_set_context_flags']==0x10010 and j['probe_integer_control_set_context_flags']==0x10003
  assert j['seed_addresses']==addresses and not j['observed_native_grid_seed_writes']
  assert all(row['seed_words']==[0]*5 for row in j['global_captures'])
  entries=[r for r in j['global_captures']if r['label']=='grid_ctor_entry'];returns=[r for r in j['global_captures']if r['label']=='grid_ctor_return']
  assert len(entries)==len(returns)==1;entry=entries[0];returned=returns[0]
  assert entry['eip']==0x430290 and returned['eip']==0x43f124 and entry['tid']==returned['tid']
  assert entry['ecx']==entry['grid_pointer']==returned['eax']
  grid=bytes.fromhex(returned['grid_bytes']);assert len(grid)==0x110
  written=[0,4,8,12,16,20,24,28,32,36]+list(range(0xa0,0x110,4))
  assert all(struct.unpack_from('<I',grid,o)[0]==0 for o in written)
  watches=[addresses[i]for i in j['watched_word_indices']];covered.update(j['watched_word_indices'])
  assert j['watched_addresses']==watches and j['watch_arms']
  for row in j['watch_arms']:
   assert row['addresses']==watches and row['requested_dr7']==row['observed_dr7'] and row['observed_dr6']&15==0
  probes=j['isolated_post_cutoff_watch_calibration_probes'];assert len(probes)==4
  for index,row in enumerate(probes):
   assert row['index']==index and row['address']==watches[index] and row['dr6']&15==1<<index
   assert row['value']==row['observed_bits']==0 and row['eip']==0x76fae7
  assert j['observation_cutoff']['write_count']==0 and j['observation_cutoff']['seed_words']==[0]*5
  captures.append(dict(directory=name,capture_sha256=sha(raw),script_sha256=j['script_sha256'],
   words=j['watched_word_indices'],armed_threads=len(j['watch_arms']),native_writes=0,calibrated_probes=4,
   constructor_entry=entry['eip'],constructor_return=returned['eip'],runtime_pins=j['runtime_pins']))
 assert covered==set(range(5))
 pins=[]
 for start,end in [(0x430290,0x430320),(0x869c30,0x869c50),(0x43f112,0x43f12a),(0x76fae1,0x76fae7),(0x43fcf1,0x43fd5a)]:
  a=L.IMAGE.at(start,end-start);b=L.ACCEPTED.at(start,end-start);assert a==b
  pins.append(dict(address=start,length=len(a),bytes_hex=a.hex(),sha256=sha(a)))
 out=dict(schema=1,scope=__doc__,native_exe_sha256=L.pe.ACCEPTED_SHA,original_exe_sha256=L.pe.ORIGINAL_SHA,
  source_identities_sha256='aa96125b653b33c0706e240b9bedec545698659a6f59181e9017e4b2e601632d',
  addresses=addresses,words=[0]*5,captures=captures,all_five_words_watched=True,code_pins=pins)
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'hardware-grid-seed-reference.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(captures=2,words=5,calibrated_probes=8,reference_sha256=sha(raw))))
if __name__=='__main__':main()

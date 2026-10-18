"""Read-only validation of two owned native byte-watch captures.
The sealed observed byte is diagnostic reference evidence, never a general
hardware default for the64-bit renderer policy.
"""
from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 summaries=[];script=HERE/'capture_request_mode.py'
 for name in ['capture-01','capture-02']:
  path=HERE/name/'capture.json';d=json.loads(path.read_text())
  assert d['schema']==3 and d['reached_request_mode_read'] and d['completed_source_return'] and d['completed_grid_constructor'] and d['completed_session_constructor']
  assert d['original_error']is None and d['original_archives_preserved'] and d['folder_redirection_observed'] and d['temporary_process_only_breakpoints'] and d['fp_state_never_written_by_debugger']
  assert d['script_sha256']==digest(script)
  assert d['source_exe_sha256']=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'==d['runtime_pins']['AA.exe']
  assert Path(d['owned_process']['exe']).resolve()==(HERE/name/'runtime/AA.exe').resolve()
  personal=[x for x in d['events']if x.get('folder_redirection_csidl')==5]
  assert personal and all(x['verified']and Path(x['destination']).resolve()==(HERE/name/'personal').resolve()for x in personal)
  assert d['request_mode_addresses']==[0xb39930,0xb39931,0xb39932,0xb39933]==d['watched_addresses']
  arms=d['watch_arms'];assert arms
  for a in arms:
   assert a['addresses']==d['watched_addresses'] and a['requested_dr7']==0x11110455
   assert a['observed_dr7']&0xffff00ff==0x11110455&0xffff00ff and a['observed_dr6']&15==0
  first=d['global_captures'][0];assert first['label']=='create_process_before_program_execution' and first['request_mode_bytes']==[0,0,0,0]
  writes=d['observed_native_request_mode_writes'];assert [x['eip']for x in writes]==[0x417a01,0x417a07,0x417a0d,0x417a13,0x41970f]
  assert [x['hit_slots']for x in writes]==[[0],[1],[2],[3],[0]]
  assert all(x['esi']==0xb39690 for x in writes)
  assert all(x['request_mode_bytes']==[0,0,0,0]for x in writes[:4]) and writes[-1]['request_mode_bytes']==[1,0,0,0]
  cutoff=d['observation_cutoff'];assert cutoff['eip']==0x45b628 and cutoff['write_count']==5 and cutoff['request_mode_bytes']==[1,0,0,0]
  read=[x for x in d['global_captures']if x['label']=='request_mode_first_read'];assert len(read)==1 and read[0]['actual_uvar108']==1 and read[0]['request_mode_bytes'][0]==1
  probes=d['isolated_post_cutoff_watch_calibration_probes'];assert len(probes)==4
  for i,p in enumerate(probes):assert p['index']==i and p['address']==d['watched_addresses'][i] and p['dr6']&15==1<<i and p['eip']==0x4b3204 and p['observed_bits']==p['value']==([1,0,0,0][i])
  cleanup=d['cleanup'];assert cleanup['wait_result']==0 and cleanup['exit_code']==0 and cleanup['exit_process_event_observed'] and not cleanup['errors']
  summaries.append(dict(path=str(path.relative_to(HERE)),sha256=digest(path),native_threads_armed=len(arms),native_write_events=5,post_cutoff_probe_events=4,byte_at_first_read=1,cleanup='verified-owned-process terminated and reaped'))
 policy=HERE/'native-request-policy.json';native=json.loads(policy.read_text());assert len(native['rows'])==108 and len(native['instructions'])==48
 out=dict(schema=1,scope=__doc__,captures=summaries,capture_producer_sha256=digest(script),native_projection_sha256=digest(policy),
  legacy_projection='Actual native wide-memory path shifts prior32-bit MiB field; constructor begins0',
  modern_backend='Current u64 physical-total query, ceiling MiB then same unsigned <700 predicate; no fixed allocation/VRAM/streaming cap',
  no_general_observed_byte_default=True,no_later_immutability_claim=True)
 path=HERE/'hardware-request-mode-reference.json'
 if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
 else:assert json.loads(path.read_text())==out
 print(json.dumps(dict(captures=2,threads=[x['native_threads_armed']for x in summaries],native_writes=10,calibration_probes=8,sha256=digest(path))))
if __name__=='__main__':main()

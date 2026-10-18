"""Read-only independent replay of frozen Sun startup hardware receipt."""
from pathlib import Path
import hashlib,json
ROOT=Path(__file__).resolve().parents[2]
AREA=ROOT/'research/skirmish-20261016-sun-startup-capture'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def doc(p):return json.loads(p.read_bytes())
capture=AREA/'capture-01/capture.json';c=doc(capture)
assert sha(capture)=='0de2475b1f9eeb5dc99c7221789a3a8357749174a3cf20b12a501a2ddba4ccf1'
assert sha(AREA/'capture_sun_startup.py')==c['script_sha256']=='93f9ad79892216253d7c88d314480035b600eca01429f59b07f7390133dbb4bf'
ref=AREA/'sun-startup-reference.json';r=doc(ref)
assert sha(ref)=='615e66797737ef5a8503625a4047119751659b6460e39802fac966b7218e8f33'
assert c['original_error'] is None and c['completed_source_return'] and c['completed_session_constructor']
assert c['cleanup']['wait_result']==c['cleanup']['exit_code']==0
assert c['cleanup']['exit_process_event_observed'] and not c['cleanup']['errors']
assert c['folder_redirection_observed'] and c['original_archives_preserved']
assert c['set_context_flags']==0x10001 and c['debug_register_set_context_flags']==0x10010
assert c['probe_integer_control_set_context_flags']==0x10003 and c['fp_state_never_written_by_debugger']
assert c['watched_addresses']==[0xb2814c,0xb28150,0xb28154]
assert len(c['watch_arms'])==62 and c['observed_native_sun_writes']==[]
for arm in c['watch_arms']:
 assert arm['addresses']==c['watched_addresses'] and arm['observed_dr7']&255==0x15
 assert arm['observed_dr7']&0xffff00ff==arm['requested_dr7']&0xffff00ff
 for slot in range(3):assert arm['observed_dr7']>>(16+4*slot)&15==0xd
 assert arm['observed_dr7']>>28&15==0
assert c['observation_cutoff']['sun_words']==[0]*3
assert all(row['sun_words']==[0]*3 for row in c['global_captures'])
probes=c['isolated_post_cutoff_watch_calibration_probes'];assert len(probes)==3
for slot,p in enumerate(probes):
 assert p['index']==slot and p['address']==0xb2814c+slot*4
 assert p['dr6']&15==1<<slot and p['eip']==0x76fae7 and p['value']==p['observed_bits']==0
assert r['observed_sun_constructor'] is False
assert not any(row['label'].startswith('sun_constructor') for row in c['global_captures'])
manifest=AREA/'capture-01/runtime-files.json';assert sha(manifest)==r['runtime_manifest_sha256']
files=doc(manifest)['files'];assert len(files)==54
for f in files:assert sha(AREA/'capture-01/runtime'/f['path'])==f['source_sha256']==f['copy_sha256']
assert doc(AREA/'post-process-inventory.json')==[]
out=dict(schema=1,accepted=True,blocking_findings=[],scope='Independent narrow tracer diff and read-only frozen hardware receipt review only; not a Rust Sun binder review.',capture_sha256=sha(capture),reference_sha256=sha(ref),armed_threads=62,verified_support_files=54,native_pre_cutoff_writes=0,post_cutoff_calibration_probes=3,observed_sun_constructor=False,normal_cleanup=True,limits=['Only initialized raw .data seed through source completion and Session construction was observed.','Sun constructor was NOT reached; no map, battle or perpetual immutability claim.','Three same-value MOV calibration writes occur after the sealed observation cutoff.','Inherited CreateProcess handles persist to bounded Python exit; normal cleanup and empty final owned-game inventory were checked.'],review_script_sha256=sha(Path(__file__)))
target=AREA/'independent-hardware-review.json'
if target.exists():assert doc(target)==out
else:target.write_text(json.dumps(out,indent=2)+'\n')
print(json.dumps(dict(accepted=True,report_sha256=sha(target))))

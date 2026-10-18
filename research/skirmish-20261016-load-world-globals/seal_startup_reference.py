"""Validate the two bounded Windows traces and seal their startup seed."""
from pathlib import Path
import hashlib
import json
import os

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
NATIVE=ROOT.parent/'reconstruction/native-game/runtime-expanded-v14'
SOURCE_FINGERPRINT='aa96125b653b33c0706e240b9bedec545698659a6f59181e9017e4b2e601632d'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def write(path,doc):path.write_text(json.dumps(doc,indent=2)+'\n')
def one(rows,label):
    matches=[v for v in rows if v['label']==label]
    assert len(matches)==1,(label,len(matches))
    return matches[0]

def main():
    original_path=ROOT/'research/development10/native-source-capture-03/capture.json'
    original=json.loads(original_path.read_bytes())
    static_path=ROOT/'research/skirmish-20261016-live-map-startup/global-point-static-audit.json'
    static=json.loads(static_path.read_bytes())
    assert static['original']['containing_sections']==static['v14']['containing_sections']
    section=static['v14']['containing_sections'][0]
    assert section['full_shape_span_in_zero_filled_tail']
    results=[];coverage=set()
    for name,words in [('capture-01',[0,1,2,3]),('capture-02',[2,3,4,5])]:
        folder=HERE/name;path=folder/'capture.json';c=json.loads(path.read_bytes())
        assert c['completed_source_return'] and c['completed_session_constructor']
        assert c['original_error'] is None and c['cleanup']['wait_result']==0
        assert c['cleanup']['exit_code']==0 and c['cleanup']['exit_process_event_observed']
        assert not c['cleanup']['errors']
        assert c['folder_redirection_observed'] and c['original_archives_preserved']
        assert c['source_exe_sha256']==original['source_exe_sha256']
        assert c['runtime_pins']==original['runtime_pins']
        assert c['original_archives_sha256']==original['original_archives_sha256']
        assert c['script_sha256']==sha(HERE/'capture_startup_globals.py')
        assert c['watched_word_indices']==words and len(c['watch_arms'])>0
        assert c['observed_native_point_writes']==[]
        for arm in c['watch_arms']:
            assert arm['addresses']==[0xb3a8c4+4*i for i in words]
            assert arm['requested_dr7']&0xffff00ff==arm['observed_dr7']&0xffff00ff
        assert len(c['isolated_post_cutoff_watch_calibration_probes'])==4
        for i,p in enumerate(c['isolated_post_cutoff_watch_calibration_probes']):
            assert p['index']==i and p['dr6']&15==1<<i
            assert p['value']==p['observed_bits']==0 and p['eip']==0x76fae7
        assert c['observation_cutoff']['point_words']==[0]*6
        for row in c['global_captures']:assert row['point_words']==[0]*6
        before=one(c['global_captures'],'create_process_before_program_execution')
        assert before['session_pointer']==0 and before['session_phase'] is None
        loaded=one(c['global_captures'],'source_return')
        assert loaded['session_pointer']==0 and loaded['session_phase'] is None
        ctor=one(c['global_captures'],'session_ctor_phase_stored')
        returned=one(c['global_captures'],'session_ctor_return')
        assert ctor['session_pointer']==returned['session_pointer']!=0
        assert ctor['session_phase']==returned['session_phase']==0
        for label in ['source_entry','means_entry','values_entry','source_return']:
            a=one(c['captures'],label);b=one(original['captures'],label)
            for field in ['x87_control','x87_status','mxcsr','crt_sse2_global','globals','manager_means']:
                if field in b:assert a[field]==b[field],(name,label,field)
            assert {k:v['bits'] for k,v in a['settings'].items()}=={k:v['bits'] for k,v in b['settings'].items()}
        assert one(c['captures'],'source_return')['actors']==one(original['captures'],'source_return')['actors']
        files=[]
        for source in NATIVE.rglob('*'):
            if not source.is_file():continue
            relative=source.relative_to(NATIVE)
            if any(p.lower() in ['logs','userdata','saves','saved games','profiles'] for p in relative.parts):continue
            copied=folder/'runtime'/relative
            assert copied.is_file() and not os.path.samefile(source,copied)
            source_hash=sha(source);copied_hash=sha(copied)
            assert source_hash==copied_hash,(name,str(relative),'runtime support bytes changed')
            files.append({'path':str(relative).replace('\\','/'),'source_sha256':source_hash,'copy_sha256':copied_hash})
        assert len(files)>0
        manifest=folder/'runtime-files.json';write(manifest,{'schema':1,'files':files})
        coverage.update(words)
        results.append({'capture_sha256':sha(path),'runtime_manifest_sha256':sha(manifest),
                        'word_indices':words,'armed_threads':len(c['watch_arms']),
                        'native_writes_before_cutoff':0,'calibration_probes':4,
                        'completed_source_return':True,'completed_session_constructor':True,
                        'cleanup_succeeded':True,'owned_pid':c['owned_process']['pid']})
    assert coverage==set(range(6))
    receipt={'schema':1,'point_address':0xb3a8c4,'words':[0]*6,
             'source_identities_sha256':SOURCE_FINGERPRINT,
             'runtime_pins':original['runtime_pins'],
             'baseline_source_capture_sha256':sha(original_path),
             'pe_zero_fill_section':section,'static_audit_sha256':sha(static_path),
             'tracer_sha256':sha(HERE/'capture_startup_globals.py'),
             'native_instruction_proof_sha256':sha(HERE/'native-instructions.json'),
             'captures':results,'all_six_words_watched_across_two_runs':True,
             'scope':'Initial PE zero-filled native global carried unchanged through actual source completion and KKC_Session constructor. Reusable only as this pinned reference startup seed for the known ported load path; not perpetual native immutability, a first-map observation, or a battle test.'}
    path=HERE/'point-startup-reference.json';write(path,receipt)
    print(json.dumps({'reference_sha256':sha(path),'captures':results,'words':receipt['words']}))

if __name__=='__main__':main()

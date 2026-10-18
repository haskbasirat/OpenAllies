"""Read-only verification/sealing of existing actual Windows direction capture.
No process is launched, debugger action performed, image patched or floating
environment changed. The native captured table is compared byte-for-byte to
all three prior original/v14 oracle tables; the observed CW, not table equality
or source loading precision, identifies the admitted static-initializer PC.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
def sha(b):return hashlib.sha256(b).hexdigest()
def main():
 base=ROOT/'research/development10/native-directions-capture-02'
 raw=(base/'capture.json').read_bytes();assert sha(raw)=='6e674da7f51963b98e3331a583150f5d5bf760c61ae9bac66322cd7ef66dc18e'
 capture=json.loads(raw);table=(base/'actual-direction-table.bin').read_bytes()
 assert len(table)==16384 and sha(table)=='a0afa5e1cb34b5caedeadf9ced0020bdf3a8b4a5f553da33fe80b018176a1c02'
 assert capture['schema']==2 and capture['completed_direction_return'] and capture['original_error']is None
 assert capture['fp_state_never_written_by_debugger'] and capture['temporary_process_only_breakpoints']
 assert capture['cleanup']['exit_process_event_observed'] and not capture['cleanup']['errors']
 assert all(n==0 for n in capture['cleanup']['manual_suspend_counts_at_termination'].values())
 entry,returned=capture['captures'];assert entry['label']=='direction_entry' and returned['label']=='direction_return'
 assert entry['eip']==0x9c8680 and returned['eip']==0x96ecd4
 assert entry['thread_id']==returned['thread_id'] and returned['esp']==entry['esp']+4
 for c in [entry,returned]:
  assert c['x87_control']==c['legacy_x87_control']==c['fxsave_x87_control']==0x27f
  assert int.from_bytes(bytes.fromhex(c['raw_legacy_float_save'])[:2],'little')==0x27f
  assert int.from_bytes(bytes.fromhex(c['raw_fxsave'])[:2],'little')==0x27f
 assert returned['directions']['count']==2048 and returned['directions']['bytes']==len(table)
 assert returned['directions']['sha256']==sha(table)
 assert capture['source_exe_sha256']==L.pe.ACCEPTED_SHA
 assert sha((base/'runtime/AA.exe').read_bytes())==L.pe.ACCEPTED_SHA
 assert sha((ROOT/'research/development10/capture_native_directions.py').read_bytes())==capture['script_sha256']
 tables=(ROOT/'research/phase5-actors/direction-tables.bin').read_bytes()
 assert len(tables)==3*16384 and sha(tables)=='20826310088f1a222dd5e480efe7697a8e47814f7f5743175b26ca6238dc0e95'
 differences=[]
 for offset in [0,16384,32768]:differences.append(sum(table[i:i+8]!=tables[offset+i:offset+i+8]for i in range(0,16384,8)))
 assert differences==[433,0,0]
 pins=[]
 for start,end in [(0x9c8680,0x9c869b),(0x408ed0,0x408f4a),(0x408d50,0x408dde)]:
  a=L.IMAGE.at(start,end-start);b=L.ACCEPTED.at(start,end-start);assert a==b
  pins.append(dict(address=start,length=len(a),bytes_hex=a.hex(),sha256=sha(a)))
 assert bytes.fromhex(pins[0]['bytes_hex'])[:15]==bytes.fromhex('6800080000b9909ab300e84108a4ff')
 document=dict(schema=1,scope=__doc__,capture_sha256=sha(raw),capture_script_sha256=capture['script_sha256'],
  original_exe_sha256=L.pe.ORIGINAL_SHA,native_exe_sha256=L.pe.ACCEPTED_SHA,
  native_table_sha256=sha(table),oracle_tables_sha256=sha(tables),
  entry_address=entry['eip'],return_address=returned['eip'],startup_control_word=entry['x87_control'],
  table_count=2048,table_bytes=len(table),startup_precision='Double53',
  differing_vectors=dict(zip(['Single24','Double53','Extended64'],differences)),code_pins=pins)
 encoded=(json.dumps(document,indent=1)+'\n').encode();output=HERE/'hardware-direction-reference.json'
 if '--write'in sys.argv:output.write_bytes(encoded)
 else:assert output.read_bytes()==encoded
 print(json.dumps(dict(native_vectors=2048,matching_precision='Double53',pc24_vectors_differ=433,
  code_windows=len(pins),capture_sha256=sha(raw),reference_sha256=sha(encoded))))
if __name__=='__main__':main()

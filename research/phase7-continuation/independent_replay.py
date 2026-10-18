"""Independent rerun of frozen continuation and original ActionGo fixtures."""
from pathlib import Path
import json,sys,importlib.util,hashlib,time
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 ledger=json.loads((HERE/'findings.json').read_text())
 files=dict(ledger['source']);files.update(ledger['dependencies'])
 # README/evidence metadata may be clarified separately without changing code.
 for name in ['native_continuation.py','native_offset.py','native-continuation.json','native-offset.json']:
  files[str((HERE/name).relative_to(ROOT)).replace('\\','/')]=H(HERE/name)
 for path,digest in files.items():assert H(ROOT/path)==digest,path
 out=HERE/'independent-output';out.mkdir(exist_ok=True);runs=[]
 for name,filename in [('native_offset','native-offset.json'),('native_continuation','native-continuation.json')]:
  spec=importlib.util.spec_from_file_location('review_'+name,HERE/(name+'.py'));module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);module.OUT=out;t=time.monotonic();module.main()
  actual=json.loads((out/filename).read_text());expected=json.loads((HERE/filename).read_text());assert actual==expected,filename
  runs.append({'producer':name+'.py','identical_json':True,'cases':len(actual['cases']),'seconds':time.monotonic()-t,'output':str((out/filename).relative_to(ROOT)).replace('\\','/'),'sha256':H(out/filename)})
 sys.path.insert(0,str(ROOT/'research/phase5-navigation'));import native_oracle as old
 gold=json.loads((ROOT/'research/phase5-navigation/native-navigation-fixtures.json').read_text());pe=old.Pe(ROOT.parent/'AA.exe',gold['original_sha256'])
 for row in gold['fixtures']:assert old.Machine(pe,row['input']).run()==row['output'],row['input']['name']
 for path,digest in files.items():assert H(ROOT/path)==digest,path
 p=HERE/'independent-native-replay.json';p.write_text(json.dumps({'passed':True,'source_sha256':files,'runs':runs,'old_path_action_cases':len(gold['fixtures']),'note':'Visited-address windows retain producer seam-entry labeling; equality does not claim intercepted instructions executed.'},indent=2)+'\n');print(H(p))
if __name__=='__main__':main()

"""Fresh immutable native replay, recording every frozen fixture hash."""
from pathlib import Path
import sys,json,subprocess,hashlib
HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 reports=[]
 for name in ['fx_graph','fx_command','bone_ordinal','fx_enums']:
  fixture=HERE/('native-'+name.replace('_','-')+'.json');before=sha(fixture)
  run=subprocess.run([sys.executable,str(HERE/('native_'+name+'.py'))],capture_output=True,text=True,check=True)
  report=json.loads(run.stdout);assert before==sha(fixture)==report['sha256'];reports.append(report)
 out=dict(status='passed',native_executions=sum(r['runs'] for r in reports),fixtures=reports)
 (HERE/'author-native-replay.json').write_text(json.dumps(out,indent=2)+'\n')
 print(json.dumps(out))
if __name__=='__main__':main()

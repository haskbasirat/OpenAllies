"""Immutable fresh original/v14 replay; all author fixture hashes must remain fixed."""
from pathlib import Path
import hashlib,json,subprocess,sys
HERE=Path(__file__).resolve().parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 rows=[]
 for name in ('special_classes','special_tail'):
  fixture=HERE/('native-'+name.replace('_','-')+'.json');before=sha(fixture)
  result=subprocess.run([sys.executable,str(HERE/('native_'+name+'.py'))],capture_output=True,text=True,check=True)
  assert before==sha(fixture)
  rows.append(json.loads(result.stdout))
 report=dict(status='passed',native_executions=sum(row['runs'] for row in rows),producers=rows)
 path=HERE/'author-native-replay.json';path.write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps(dict(status='passed',runs=report['native_executions'],sha256=sha(path))))
if __name__=='__main__':main()

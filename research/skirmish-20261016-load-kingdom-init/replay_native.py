"""Run all author native checks without regenerating their fixtures."""
from pathlib import Path
import subprocess,hashlib,json,sys
HERE=Path(__file__).resolve().parent
JOBS=[('native_team_leaves.py','native-team-leaves.json',72),
 ('native_world_registry_ctor.py','native-world-registry-constructor.json',6),
 ('native_color_ordinal.py','native-color-ordinal.json',48),
 ('native_registration.py','native-registration.json',96),
 ('native_relations_masks.py','native-relations-masks.json',96),
 ('native_actual_kingdom.py','native-actual-kingdom.json',6)]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 before={fixture:sha(HERE/fixture) for _,fixture,_ in JOBS};results=[];pins={}
 for script,fixture,runs in JOBS:
  result=subprocess.run([sys.executable,str(HERE/script)],cwd=HERE.parents[1],capture_output=True,text=True)
  assert result.returncode==0,(script,result.stdout,result.stderr)
  assert sha(HERE/fixture)==before[fixture]
  doc=json.loads((HERE/fixture).read_text())
  for pin in doc['instructions']:
   if pin['va'] in pins:assert pins[pin['va']]==pin['bytes']
   pins[pin['va']]=pin['bytes']
  results.append(dict(script=script,fixture=fixture,native_runs=runs,pins=len(doc['instructions']),
      producer_sha256=sha(HERE/script),fixture_sha256=before[fixture],returncode=0,stdout=result.stdout.strip()))
 doc=dict(schema=1,status='passed',native_runs=sum(v[2] for v in JOBS),unique_executed_instruction_pins=len(pins),
  fixtures_preserved=True,checks=results)
 path=HERE/'author-native-replay.json';path.write_text(json.dumps(doc,indent=2)+'\n')
 print(json.dumps(dict(status=doc['status'],native_runs=doc['native_runs'],unique_pins=len(pins),sha256=sha(path))))
if __name__=='__main__':main()

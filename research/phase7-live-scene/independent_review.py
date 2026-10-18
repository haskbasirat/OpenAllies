"""Independent review replay. Does not replace producer logs or capture assets."""
from pathlib import Path
import hashlib,json,os,subprocess,re
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 producer=json.loads((OUT/'validation-record.json').read_text());expected=producer['files'];pins={}
 for file,digest in expected.items():
  p=ROOT/file
  if file=='crates/aa-client/src/live_scene.rs':
   s=p.read_text();start=s.index('#[cfg(test)]\nmod independent_review_tests');end=s.index('\nimpl ActorVisual',start)
   base=s[:start].rstrip('\n')+'\n'+s[end:].lstrip('\n');base=base.rstrip('\n')+'\n'
   assert hashlib.sha256(base.encode()).hexdigest()==digest,'Production changed outside reviewer test block'
  else:assert H(p)==digest,file
  pins[file]=H(p)
 env=os.environ.copy();env['AA_GAME_ROOT']=str(ROOT.parent)
 commands=[]
 for name,filter in [('review-live.log','live_'),('review-gpu.log','gpu::tests'),('review-picking.log','inspection::tests')]:
  cmd=['cargo','test','-p','aa-client','--release',filter,'--','--include-ignored','--nocapture'];p=subprocess.run(cmd,cwd=ROOT,env=env,text=True,capture_output=True)
  (OUT/name).write_text(p.stdout+p.stderr);assert p.returncode==0,p.stdout+p.stderr
  matches=re.findall(r'test result: ok\. (\d+) passed',p.stdout)
  commands.append({'command':cmd,'passed':sum(map(int,matches)),'exit_code':p.returncode,'log':name,'sha256':H(OUT/name)})
  print(name,commands[-1]['passed'],flush=True)
 captures=[]
 logs=(OUT/'review-live.log').read_text();match=re.search(r'Independent live snapshot captures: (.+)',logs)
 if match:
  folder=Path(match.group(1).strip())
  for p in sorted(folder.glob('*.png')):captures.append({'path':str(p),'sha256':H(p)})
 # Fresh producer hardware captures have their own unique process/timestamp path.
 final={p:H(ROOT/p)for p in pins}
 assert pins==final,'Reviewed source/artifact changed during replay'
 result={'schema':1,'passed':True,'reviewer':'navigation independent review','scope':'Live actor presentation only; no runtime snapshot source, simulation, full-match, spawn asset loading or native gameplay picking claim.','findings':[],'source_files_sha256':{k:v for k,v in pins.items()if k.startswith('crates/')},'producer_validation_sha256':H(OUT/'validation-record.json'),'review_addition':'Only cfg(test) independent_review_tests was added with owner agreement. Removing that block and its adjacent blank line exactly reproduces the original producer live_scene.rs SHA256. Production code is unchanged.','commands':commands,'new_live_apply_captures':captures,'checked':['Map native IDs bind all retained mesh instances after exact empty-draw compaction.','All caller validation precedes GPU writes; CPU picking only publishes after GPU acceptance.','Fresh original-map end-to-end test covers invalid later actor, accepted move/hide, duplicate/decreasing sequence, duplicate actor IDs, invalid GPU slot after staged picking, unknown actor and stale renderer epoch.','GPU pixels and CPU positions/visibility remain unchanged after rejected snapshots.','Opaque transforms/colors retain cached command bundle; transparent transforms and visibility invalidate ordering.','Picking uses updated bounds/inverses and physical viewport aspect with logical pointer coordinates.','Per-instance color attribute9 does not overlap terrain UV8; shader fallback to global team color preserved.','Scene upload advances epoch only after successful staged upload.'],'limits':['Current main retains and attaches loaded scenes but does not supply real runtime ActorVisual snapshots.','Planar facing only; event-selected per-actor animation, pitch/roll and model-spawn bindings remain separate.','Hardware/driver loss is not a transactional rollback guarantee.','Low-level GPU tests and original assets ran locally on this GPU; no cross-device/remote internet test.']}
 (OUT/'independent-review.json').write_text(json.dumps(result,indent=2)+'\n');print({'passed':True,'sha256':H(OUT/'independent-review.json')},flush=True)
if __name__=='__main__':main()

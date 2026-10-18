"""Independent readonly Sim replay, with strict page-zero checks added only in-memory. Does not rewrite author evidence."""
from pathlib import Path
import sys,importlib.util,json,hashlib
R=Path.cwd();D=R/'research/development10/live-sim'
sys.path.insert(0,str(R.parent/'reconstruction/tools/python-packages'))
sys.path.insert(0,str(R/'research/phase8-restoration'));import null_guard
manifest=json.loads((D/'review-freeze-v2.json').read_text());H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
for file,expected in manifest['production_and_tests'].items():assert H(R/file)==expected,(file,H(R/file),expected)
for file,expected in manifest['evidence'].items():assert H(D/file)==expected,file
scripts=['native_live_sim.py','native_borrowed_driver.py','native_created_world.py','native_admin_end_read_prefix.py']
for i,name in enumerate(scripts):
 spec=importlib.util.spec_from_file_location('independent_sim_'+str(i),D/name);m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
 for cls_name in ['Machine','Constructor','Reset']:
  cls=getattr(m,cls_name,None)
  if cls is None:continue
  init=cls.__init__;run=cls.run
  def wrapped_init(self,*args,_init=init,**kwargs):_init(self,*args,**kwargs);null_guard.install(self)
  def wrapped_run(self,*args,_run=run,**kwargs):
   out=_run(self,*args,**kwargs);assert not self.null_accesses,self.null_accesses;return out
  cls.__init__=wrapped_init;cls.run=wrapped_run
 if hasattr(m.h,'VISITED'):m.h.VISITED.clear() # each script's own fresh-process pin accumulator
 print('readonly',name,flush=True);m.main()
for file,expected in manifest['evidence'].items():assert H(D/file)==expected,file
print('Independent native replay and source/evidence pins preserved. Strict null guards added in-memory only.')


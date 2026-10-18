"""Independent navigation-worker audit of native TERRAIN input and static flags."""
from pathlib import Path
import contextlib,hashlib,importlib,json,re,subprocess,sys
ROOT=Path(__file__).resolve().parents[2];SOURCE=Path(__file__).parent
OUT=SOURCE/'independent-terrain-output'
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
def main():
 OUT.mkdir(exist_ok=True);sys.path.insert(0,str(SOURCE));commands=[];regenerated=[]
 ledger=json.loads((SOURCE/'terrain-findings.json').read_text())
 for name,digest in ledger['source_sha256'].items():assert sha(ROOT/name)==digest,name
 for name,filename in [('native_terrain','native-terrain.json'),('original_map_terrain','native-real-map-terrain.json')]:
  module=importlib.import_module(name);module.OUT=OUT
  with (OUT/(name+'.log')).open('w') as log,contextlib.redirect_stdout(log):module.main()
  assert json.loads((OUT/filename).read_text())==json.loads((SOURCE/filename).read_text()),filename
  regenerated.append({'fixture':filename,'sha256':sha(SOURCE/filename),'reproduced':True})
 from native_terrain import original,modified
 from capstone import Cs,CS_ARCH_X86,CS_MODE_32
 md=Cs(CS_ARCH_X86,CS_MODE_32);ranges={}
 for name,address,size in [('layout',0x5565e1,0x87),('loader_through_tile_input',0x557ad0,0x43b),('passability',0x54ee20,0x3e),('recompute_all',0x552c50,0x40),('clear_region',0x54ed60,8)]:
  raw=original.range(address,size);assert raw==modified.range(address,size)
  path=OUT/(name+'.txt');path.write_text('\n'.join(f'{i.address:08x} {i.bytes.hex():24} {i.mnemonic} {i.op_str}'for i in md.disasm(raw,address))+'\n')
  ranges[name]={'start':hex(address),'length':size,'sha256':hashlib.sha256(raw).hexdigest(),'v14_identical':True,'assembly':str(path.relative_to(ROOT))}
 tests=0
 for args,name in [(['cargo','test','-p','aa-mapworld','--test','terrain','--test','terrain_corpus','--','--include-ignored','--nocapture'],'rust-tests.log'),(['cargo','clippy','-p','aa-mapworld','--all-targets','--','-D','warnings'],'clippy.log')]:
  run=subprocess.run(args,cwd=ROOT,text=True,stdout=subprocess.PIPE,stderr=subprocess.STDOUT);log=OUT/name;log.write_text(run.stdout)
  commands.append({'command':args,'exit_code':run.returncode,'log':str(log.relative_to(ROOT)),'sha256':sha(log)})
  assert run.returncode==0,run.stdout
  if name=='rust-tests.log':tests=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout)))
 assert tests==5
 # Bind the exact frozen source reviewed above, and reject concurrent changes.
 for name,digest in ledger['source_sha256'].items():assert sha(ROOT/name)==digest,name
 report={'schema':1,'passed':True,'reviewer':'independent navigation worker','findings':[],
  'scope':'Native GMap v6/v7 tile input, explicit authored-definition binding and carried fallback, static passability, source corpus and untouched auxiliary suffix boundary',
  'producer_ledger_sha256':sha(SOURCE/'terrain-findings.json'),'reviewed_source_sha256':ledger['source_sha256'],'regenerated':regenerated,'native_ranges':ranges,
  'native_counts':{'layouts':20,'tile_loads':32,'passability_cases':1048,'missing_definition_sequences':8,'actual_map_layouts':80,'actual_source_sample_cells':720},
  'source_corpus_maps':80,'source_corpus_tiles':2960384,'source_fallback_cells':463557,'rust_tests':tests,'commands':commands,
  'checked_semantics':['Original resolution and ceil-then-shift layout; finite nonnegative representable domain is explicit.','V6 six-byte expansion exactly preserves ID/flag/region/byte2C fields and native zeroed unused bytes; raw input bytes retained separately.','V7 source records and source-order iteration; exact low-byte load, type-bit replacement and54EE20 land/water passability masks.','The original missing-definition branch retains the previous resolved variation, initially supplied from authored biome default; error-message presentation is an explicit oracle seam.','Native suffix boundary557F0B follows tile input and passability recomputation; auxiliary bytes and serialized region references are preserved, not published as runtime topology.','Both original native fixture files regenerated independently into a separate output directory and compared without replacing frozen fixtures.','All80 original map/header/terrain identities, definition/translation/SVar pins and2,960,384 cells checked by the source corpus test;720 actual source-order cells separately reexecuted through original loader.'],
  'limits':['Original runtime registry construction/default resolution rely on their separate evidence; this review verifies typed input use and corpus source pins.','Only stream game modefalse is executed; native region handle restoration and byte2C auxiliary counts are not exposed as a completed runtime world.','No review of concurrent water/suffix decoder, region topology construction, dynamic actor membership or full gameplay.','All3million cells are source/formula comparisons;720 are native replay samples, not3million native executions.']}
 path=SOURCE/'independent-review-terrain.json';path.write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'tests':tests,'maps':80,'sha256':sha(path)}))
if __name__=='__main__':main()

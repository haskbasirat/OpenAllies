"""Real4B3190/4B29B0/40FCA0/40EAA0 Bool byte assignments. Actual Session
E4/F4, null local Node+8 and DevMode UVar byte are explicit inputs. No game-code
seams. This covers the editor gate's reached null/local-not-playing branches,
not the positive local-player name service or an actual application binding.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 var=m.alloc(0x30);m.w8(var,c['before']);m.w8(var+1,77);m.w32(var+0x24,1,c['flags'])
 session=m.alloc(0x100);m.w32(0xb36550,session);m.w32(session+0xe4,c['e4']);m.w8(session+0xf4,c['f4'])
 node=m.alloc(0x20);m.w32(0xb36558,node)
 uvars=m.alloc(0x100);dev=m.alloc(4);m.w32(0xb39940,uvars);m.w32(uvars+0xf8,dev);m.w8(dev,c['dev'])
 allowed=m.call(0x4b29b0,var)&255
 m.call(0x4b3190,var,[c['requested']])
 return {'allowed':allowed,'current':m.r8(var),'default':m.r8(var+1)}
def main():
 rows=[]
 for pc,before,requested,flags,dev,session in itertools.product(h.PRECISION,[0,128],[0,1,128,255],[5,16],[0,128],[(0,0),(3,1),(1,1)]):
  c={'before':before,'requested':requested,'flags':flags,'dev':dev,'e4':session[0],'f4':session[1]}
  a=run('original',pc,c);b=run('v14',pc,c);assert a==b
  rows.append({'precision':pc,'input':c,'output':a})
 pins=[]
 for a,b in sorted(h.VISITED.items()):
  raw=bytes.fromhex(b);assert raw==ORIGINAL.range(a,len(raw))==h.V.range(a,len(raw));pins.append({'va':hex(a),'bytes':b})
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'executed_instructions':pins}
 p=HERE/'native-view-bool-assignment.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()

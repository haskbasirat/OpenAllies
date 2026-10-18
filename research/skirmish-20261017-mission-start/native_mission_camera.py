"""Actual4F61A0 observer/capital/missing-capital camera body.

4F5290 is an explicit local-Kingdom lookup boundary (independently ported and
tested elsewhere), and404F60 is captured log transport. Every body store and
x87 multiply/load/store/log argument conversion executes original/v14 bytes.
No complete mission or application activation is claimed.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
def one(image,pc,kind,xy,extents):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);world=m.alloc(0x300);terrain=m.alloc(0x80);kingdom=m.alloc(0x400);capital=m.alloc(0x40)
 m.w32(0xb36534,world);m.w32(world+0x30,terrain);m.w32(terrain+12,*extents);m.w32(world+0x22c,0xaa55aa55,0x55aa55aa)
 m.w32(kingdom+0x1f8,capital if kind==2 else 0);m.w32(capital+0x20,*xy)
 m.seams[0x4f5290]=lambda _:m.ret(value=0 if kind==0 else kingdom)
 logs=[]
 def log(_):
  line=m.arg(2);row={'line':line}
  if line!=0x136:row['double_arguments']=[m.arg(5)|(m.arg(6)<<32),m.arg(7)|(m.arg(8)<<32)]
  logs.append(row);m.ret()
 m.seams[0x404f60]=log;m.call(0x4f61a0,world)
 assert len(logs)==1 and logs[0]['line']==[0x128,0x136,0x132][kind]
 return {'look_at':[m.r32(world+0x22c),m.r32(world+0x230)],'logs':logs}
def main():
 rows=[]
 for image,pc,kind,(xy,extents)in itertools.product(('original','v14'),h.PRECISION,range(3),[
  ([0x80000000,0x3f800001],[0x43c00000,0x43bfffff]),
  ([0xc1234567,0x41234567],[0x44702000,0x44706000]),
  ([0x00800001,0x80800001],[0x00800001,0x00800003])]):
  rows.append({'image':image,'precision':pc,'input':{'kind':kind,'xy':xy,'extents':extents},'output':one(image,pc,kind,xy,extents)})
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'seams_not_executed':['0x4f5290','0x404f60'],'executed_pins':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in(0x4f5290,0x404f60)],'cases':rows}
 p=HERE/'native-mission-camera.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'cases':len(rows),'pins':len(doc['executed_pins']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()

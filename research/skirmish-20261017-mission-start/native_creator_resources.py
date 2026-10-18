"""Native4CDC70 zero-and-overlay Resource array used by510580 parameters Load.
Actual45F180 and4CDC70 execute unchanged. Stream DWORD reads and4CDBB0 typed
saved-reference transport are explicit seams; Find, CString and its warning
path are NOT executed/certified. Resource ordinals are validated explicit input
slots. Unknown references skip in this native leaf; the live source binder
currently refuses them because the required warning path is not bound.
"""
from pathlib import Path
import sys,itertools,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def run(inputs,image,pc):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(pc);seams=set();n=inputs['count'];rows=inputs['rows'];cursor=iter(rows);pending=[None]
 gd=m.alloc(0x80);m.w32(0xb36530,gd);m.w32(gd+0x38,n)
 vector=m.alloc(12);data=m.alloc(max(1,n)*4);m.w32(vector,data,n,n)
 for i in range(n):m.w32(data+4*i,0xdead0000+i)
 stream=m.alloc(16);vt=m.alloc(4);entry=m.stub_next;m.stub_next+=16;m.w32(stream,vt);m.w32(vt,entry)
 values=iter([len(rows)]+[r['word']for r in rows])
 def read(mm):assert mm.arg(1)==4;mm.w32(mm.arg(),next(values));mm.ret(pop=8)
 def ref(mm):
  row=next(cursor);p=0
  if row['index'] is not None:p=mm.alloc(0x10);mm.w32(p+12,row['index'])
  mm.w32(mm.arg(1),p);mm.ret(value=mm.arg(1))
 m.seams[entry]=read;seams.add(entry);m.seams[0x4cdbb0]=ref;seams.add(0x4cdbb0)
 m.call(0x4cdc70,0,[stream,vector])
 return [m.r32(data+4*i)for i in range(n)],seams

def main():
 inputs=[{'count':n,'rows':rows}for n,rows in [
 (0,[]),(3,[]),(3,[{'index':0,'word':123}]),
 (3,[{'index':2,'word':0x7fa00001},{'index':2,'word':0x80000000}]),
 (3,[{'index':None,'word':77},{'index':1,'word':0xffffffff}]),
 (3,[{'index':4,'word':10}]),
 (7,[]),(7,[{'index':0,'word':123}]),
 (7,[{'index':6,'word':0x7fa00001},{'index':6,'word':0x80000000}]),
 (7,[{'index':None,'word':77},{'index':1,'word':0xffffffff}]),
 (7,[{'index':8,'word':10}])]]
 cases=[];seams=set()
 for image,pc,inputs in itertools.product(('original','v14'),h.PRECISION,inputs):
  output,used=run(inputs,image,pc);seams|=used;cases.append({'image':image,'precision':pc,'input':inputs,'output':output})
 pins=[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'seams_not_executed':[hex(a)for a in sorted(seams)],
  'executed_instructions':pins,'cases':cases}
 path=HERE/'native-creator-resources.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps({'cases':len(cases),'executed_pins':len(pins),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()

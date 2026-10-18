"""50D120 type1 message append and enabled1C one-time WaitWorld order.
CString construction/assignment are explicit memory/value seams; no UI flush.
Native array is provisioned sufficiently, so no resize policy is claimed.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
def run(c,image):
 h.P=ORIGINAL if image=='original' else h.V;m=h.Machine(c['precision'])
 def string(text):
  data=text.encode('utf-16le');base=m.alloc(16+len(data)+2)
  m.w32(base,0x1234,len(data)//2,len(data)//2,1);m.u.mem_write(base+16,data+b'\0\0');return base+16
 def text(ptr):
  return bytes(m.u.mem_read(ptr,m.r32(ptr-12)*2)).decode('utf-16le')
 def literal(mm):
  raw=bytearray();p=mm.arg()
  while True:
   pair=bytes(mm.u.mem_read(p,2));p+=2
   if pair==b'\0\0':break
   raw.extend(pair)
  target=mm.ecx();mm.w32(target,string(raw.decode('utf-16le')));mm.ret(4,target)
 def assign(mm):
  target=mm.ecx();value=mm.r32(mm.arg());mm.w32(target,value);mm.w32(value-4,mm.r32(value-4)+1);mm.ret(4,target)
 m.seams.update({0x401b00:literal,0x405b80:assign})
 manager=m.alloc(32);data=m.alloc(16*0x4c)
 m.w32(manager+0x10,data,len(c['prior']),16);m.w8(manager+0x1c,c['enabled'])
 for i,item in enumerate(c['prior']):m.w32(data+i*0x4c,1);m.w32(data+i*0x4c+0x48,string(item))
 for value in c['submissions']:
  word=m.alloc(4);m.w32(word,string(value));m.call(0x50d120,manager,[word])
 count=m.r32(manager+0x14)
 assert m.r32(manager+0x10)==data and count<=16
 return dict(enabled=m.r8(manager+0x1c),messages=[dict(kind=m.r32(data+i*0x4c),text=text(m.r32(data+i*0x4c+0x48))) for i in range(count)])
def main():
 rows=[]
 for precision,enabled,prior,submissions in itertools.product(h.PRECISION,[0,1,128],[[],['existing']],
    [['TriggerListUpdate'],['TriggerListUpdate','','second']]):
  c=dict(precision=precision,enabled=enabled,prior=prior,submissions=submissions)
  a=run(c,'original');b=run(c,'v14');assert a==b;rows.append(dict(input=c,output=a))
 out=dict(schema=1,scope=__doc__,rows=rows,native_executions=len(rows)*2,
  original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  seams=[dict(address=hex(a),purpose=n) for a,n in [(0x401b00,'CString literal construction'),(0x405b80,'CString assignment/refcount')]],
  instructions=[dict(address=hex(a),bytes=b) for a,b in sorted(h.VISITED.items())])
 path=HERE/'native-broadcast.json'
 if '--write' in sys.argv:path.write_text(json.dumps(out,indent=1)+'\n')
 else:assert json.loads(path.read_text())==out
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(h.VISITED),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()

"""Actual511AF0 current bool rule writer with real511A60 first-IDS lookup.
CString case-insensitive CRT comparison is an ASCII transport seam. Canonical
record definition pointers/IDS, vector storage and change counter are explicit
inputs; no source registry/Creator constructor/770280 success is claimed.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

def run(image,pc,c):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams={0x96cad6}
 def string(s):
  p=m.alloc(len(s)*2+2);m.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 def text(p):
  b=bytearray()
  while bytes(m.u.mem_read(p,2))!=b'\0\0':b+=bytes(m.u.mem_read(p,2));p+=2
  return b.decode('utf-16le')
 m.seams[0x96cad6]=lambda _:m.ret(value=int(text(m.arg()).lower()!=text(m.arg(1)).lower()))
 owner=m.alloc(0x20);data=m.alloc(max(1,len(c['records']))*12);m.w32(owner+12,data,len(c['records']),len(c['records']));m.w32(owner+0x18,c['changes'])
 for i,row in enumerate(c['records']):
  d=m.alloc(0x44);m.w32(d+8,string(row['ids']));m.w32(d+0x20,row['kind']);m.w32(data+12*i,d,0x99887766,row['word'])
 arg=m.alloc(0x44);m.w32(arg+8,string(c['name']));states=[]
 for raw in c['values']:
  m.call(0x511af0,owner,[arg,raw]);states.append({'changes':m.r32(owner+0x18),'words':[m.r32(data+12*i+8)for i in range(len(c['records']))]})
 return states

def main():
 cases=[];inputs=[]
 for records in [[{'ids':'RATED','kind':0,'word':0xaabbcc80},{'ids':'Other','kind':1,'word':0x11223344}],
                 [{'ids':'RATED','kind':1,'word':0x12345678},{'ids':'rated','kind':0,'word':0xdeadbe00}]]:
  for name,changes in itertools.product(('rated','RATED','Other','Missing'),(0,0xffffffff)):
   inputs.append({'records':records,'name':name,'changes':changes,'values':[128,128,0,1,255]})
 for image,pc,c in itertools.product(('original','v14'),h.PRECISION,inputs):
  cases.append({'image':image,'precision':pc,'input':c,'output':run(image,pc,c)})
 pins=[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a!=0x96cad6]
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'executed_instructions':pins,'seams_not_executed':['0x96cad6'],'cases':cases}
 p=HERE/'native-world-rule-bool.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'cases':len(cases),'calls':len(cases)*5,'executed_pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()

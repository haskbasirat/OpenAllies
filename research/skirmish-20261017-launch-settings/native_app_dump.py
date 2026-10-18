"""Actual784560 AppLaunchState diagnostic order, every leaf cutoff,
original/v14 allPC. Explicit raw App fields, inherited CString/allocator and
observed logging transports; +0C intentionally poisoned and never read.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));from native_launch_dump import DumpMachine,Stop,h

def run(image,pc,stop):
 m=DumpMachine(image,pc,stop);p=m.alloc(0x28);m.u.mem_write(p,b'\xcd'*0x28)
 for o,v in [(0,7),(0x10,3),(0x18,11),(0x1c,4),(0x24,2)]:m.w(p+o,v)
 for o,v in [(4,1),(0x14,0),(0x15,1),(0x16,0),(0x20,0)]:m.b(p+o,v)
 m.cs(p+8,[65,0,66,55296]);stopped=False
 try:m.call(0x784560,[m.string('not-a-marker'),p])
 except Stop:stopped=True
 return dict(events=m.events,stopped=stopped),m.visited,m.intercepted

def main():
 rows=[];visited={};seams=set()
 for pc,stop in itertools.product(h.PRECISION,[None]+list(range(11))):
  a,pa,sa=run('original',pc,stop);b,pb,sb=run('v14',pc,stop);assert a==b
  visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,stop=stop,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 data=(json.dumps(dict(schema=1,scope=__doc__,rows=rows,executions=len(rows)*2,instructions=pins,seams=[hex(a)for a in sorted(seams)]),indent=1)+'\n').encode();path=HERE/'native-app-dump.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()

"""Actual4E6DB0 positive Commander path with no profile/start-vector entries.
Normal rows execute4E39D0,4730D0 and4CDF10 without game-code seams. Required
rows stop at first4E3AD0 or401B00 localization entry, or actual current134
memory read. These are explicit service boundaries, never successful callbacks.
All source objects/vectors are supplied valid synthetic allocations; no complete
saved/source/application startup is claimed.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P
class Required(Exception):pass
def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);k=m.alloc(0x400);cmd=m.alloc(0x30);faction=m.alloc(0x60)
 stock=m.alloc(8);delta=m.alloc(8);m.w32(stock,h.f2b(5),h.f2b(8));m.w32(delta,h.f2b(2),h.f2b(3));m.w32(k+0x1c4,stock,2,2);m.w32(k+0x258,delta,2,2)
 m.w8(k+0x23a,c['reset']);m.w32(k+0x19c,9);m.w8(k+0x23b,0);m.w32(k+0x230,cmd);m.w32(cmd+4,faction);m.w32(cmd+12,h.f2b(7));m.w32(faction+0x3c,c['maximum'])
 prop=m.alloc(0x60)
 if c['property']:
  node=m.alloc(8);m.w32(node,prop,0);m.w32(faction+0x30,node)
 if c['nation']:
  nation=m.alloc(0x80);m.w32(k+0x234,nation)
  if c['nation']!=1:
   pointers=m.alloc(8);m.w32(pointers,prop,faction if c['nation']==2 else prop);m.w32(nation+0x48,pointers,2)
 cache=m.alloc(0x300);m.w32(0xb39ab0,cache);initial=m.alloc(4);maximum=m.alloc(4);experience=m.alloc(4)
 m.w32(initial,h.f2b(10));m.w32(maximum,h.f2b(50));m.w32(experience,c['experience']);m.w32(cache+0x110,initial);m.w32(cache+0x1b8,maximum);m.w32(cache+0x134,experience)
 reads=0;events=[]
 def read(u,access,a,n,v,_):
  nonlocal reads
  if a<=experience<a+n:
   reads+=1
   if c['fail_experience']:raise Required('experience_required')
 m.u.hook_add(UC_HOOK_MEM_READ,read)
 def grant(_):
  assert m.ecx()==k and [m.arg(i)for i in range(4)]==[prop,2,k,1]
  events.append('property');raise Required('property_required')
 def warning(_):events.append('warning');raise Required('warning_required')
 m.seams[0x4e3ad0]=grant;m.seams[0x401b00]=warning
 outcome='ok'
 try:m.call(0x4e6db0,k)
 except Required as e:outcome=str(e)
 return dict(outcome=outcome,stock=m.r32(stock,2),experience=m.r32(cmd+12),experience_reads=reads,status=m.r32(k+0x19c),started=m.r8(k+0x23b),events=events)
def main():
 rows=[]
 pairs=[(h.f2b(200),h.f2b(5000)),(h.f2b(-10),h.f2b(-1)),(h.f2b(200),h.f2b(25)),(0x7fc01234,h.f2b(5000))]
 for pc,(experience,maximum),reset,nation,prop,fail in itertools.product(h.PRECISION,pairs,(0,1),(0,1,2,3),(0,1),(False,True)):
  c=dict(experience=experience,maximum=maximum,reset=reset,nation=nation,property=prop,fail_experience=fail)
  a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b;rows.append(dict(precision=pc,input=c,output=a))
 doc=dict(scope=__doc__,rows=rows,executions=2*len(rows),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),required_entry_pins=[dict(va=hex(a),bytes=h.P.range(a,8).hex())for a in(0x4e3ad0,0x401b00)],
  instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())if a not in(0x4e3ad0,0x401b00)])
 p=HERE/'native-commander-start.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(doc['instructions']),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()

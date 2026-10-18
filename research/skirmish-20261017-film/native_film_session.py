"""76C310/real765E00, same Session roster/stream order; header, chunk stream,
diagnostics and attachment are explicit required services, not success claims.
Default execution verifies immutable fixture; --write is explicit generation.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P

class StopService(Exception):pass

def run(case,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc)
 session=m.alloc(0x200);resource=m.alloc(0xb4);stream=resource+0x3c
 vt=m.alloc(0x50);m.w32(stream,vt);m.w32(session+0x1c,resource)
 settings=m.alloc(0x400);compress=m.alloc(1);m.w8(compress,case['compressed'])
 m.w32(settings+0x28c,compress);m.w32(0xb39940,settings)
 world=m.alloc(0x300);m.w32(0xb36534,world if case['world_present'] else 0)
 admin=m.alloc(0x300);m.w32(0xb36558,admin)
 nodes=[];players=[];kingdom_reads=[]
 for p in case['players']:
  player=m.alloc(0x60);node=m.alloc(8);players.append(player);nodes.append(node)
  units=p['name'];header=m.alloc(16+2*(len(units)+1));text=header+16
  m.w32(header+4,len(units));m.w32(header+12,2)
  m.u.mem_write(text,b''.join(int(u).to_bytes(2,'little')for u in units)+b'\0\0')
  m.w32(player+0x20,p['id'],text);m.w8(player+0xc,p['ai']);m.w8(player+0xd,p['implicit'])
  if p['kingdom'] is not None:
   kingdom=m.alloc(0x200);m.w32(kingdom+0x188,p['kingdom']);m.w32(player+0x28,kingdom)
 for i,node in enumerate(nodes):m.w32(node,players[i],nodes[i+1] if i+1<len(nodes) else 0)
 m.w32(session+0xbc,nodes[0] if nodes else 0)
 events=[];payload=[];step=0;attached=False;flushed=False
 def event(name,**kw):
  nonlocal step
  if step==case.get('stop',-1):raise StopService(name)
  step+=1;events.append(dict(name=name,**kw))
 def text(p):
  out=[]
  while m.r32(p)&0xffff:out.append(m.r32(p)&0xffff);p+=2
  return ''.join(chr(u)for u in out)
 def header(_):
  assert m.ecx()==resource;event('header_strings');m.ret()
 def chunk(_):
  assert m.ecx()==stream
  event('chunk',chunk=text(m.arg()),compressed=m.arg(1));m.ret(8)
 def write(_):
  assert m.ecx()==stream
  data=bytes(m.u.mem_read(m.arg(),m.arg(1)));event('write',hex=data.hex());payload.append(data);m.ret(8)
 def diag(_):
  event('diagnostic',ai=m.arg(5),implicit=m.arg(6),id=m.arg(7));m.ret()
 def flush(_):
  nonlocal flushed
  assert m.ecx()==stream;event('flush');flushed=True;m.ret()
 def attach(_):
  nonlocal attached
  assert m.ecx()==admin and m.arg()==stream;event('attach');attached=True;m.ret(4)
 def stub(f):
  at=m.stub_next;m.stub_next+=16;m.u.mem_write(at,b'\xc3');m.seams[at]=f;return at
 m.w32(vt+4,stub(write));m.w32(vt+0x3c,stub(flush))
 m.seams.update({0x5a01e0:header,0x859150:chunk,0x404f60:diag,0x74d5f0:attach})
 # Observer only: real765E00 executes, including the actual global World gate.
 from unicorn import UC_HOOK_CODE
 def observe(u,at,size,_):
  if at==0x765e00:kingdom_reads.append(players.index(m.ecx()))
 m.u.hook_add(UC_HOOK_CODE,observe)
 stopped=None
 try:
  m.call(0x76c310,session);assert m.final_esp==h.STACK+4 and m.fpu_top()==0
 except StopService as e:stopped=str(e)
 return dict(events=events,payload_hex=b''.join(payload).hex(),kingdom_reads=kingdom_reads,flushed=flushed,attached=attached,stopped=stopped)

def cases():
 def player(i=7,name=None,kingdom=2,ai=0,implicit=0):return dict(id=i,name=[65,0xd800,0,66]if name is None else name,kingdom=kingdom,ai=ai,implicit=implicit)
 shapes=[[],[player()],[player(0xffffffff,[],None,255,128)],
  [player(0x81000042,[0x1234,0xdcff],0xffffffff,1,1),player(0xdeadbeef,[90],0,0,1),player(3,[81,82],None,1,0)],
  [player(8,[65]*65537,1)]]
 for players in shapes:
  for world in [False,True]:
   for compressed in [0,1]:yield dict(players=players,world_present=world,compressed=compressed)
 base=dict(players=shapes[3],world_present=True,compressed=1)
 # Every required service prefix, including all write/diagnostic/flush/attach.
 count=len(run(base,'original','Double53')['events'])
 for stop in range(count):yield dict(base,stop=stop)

def main():
 rows=[]
 for case in cases():
  for pc in h.PRECISION:
   original=run(case,'original',pc);expanded=run(case,'v14',pc);assert original==expanded
   rows.append(dict(input=case,pc=pc,output=original))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,cases=rows,
  instructions=[dict(va=hex(a),bytes=b)for a,b in sorted(h.VISITED.items())])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-session.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data,'fixture bytes differ'
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(h.VISITED),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()

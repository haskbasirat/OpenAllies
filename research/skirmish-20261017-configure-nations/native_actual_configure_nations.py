"""Actual completed-bank Nation/Faction inputs from current Firefight records.
Execute bounded50EC66..50ECFA +4E15D0/474B40 on one retained GWorld RNG per
image/PC. Commander4E0F00 is an explicit proven-constructor child seam; source
IDs/eligibility, per-record result, seed stream and A/S log counts must match
Rust's actual-owner capture. Does not authorize omitted record lifecycle.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
import native_configure_nations as base
nm=base.nm
from unicorn.x86_const import *
def run(capture,image,pc):
 m=base.machine(image,pc);events=[];rows=capture['source_rows'];names=list(dict.fromkeys(f['id']for n in rows for f in n['factions']));factions={n:m.alloc(0x40)for n in names};enabled={f['id']:f['enabled']for n in rows for f in n['factions']}
 for name,p in factions.items():m.u.mem_write(p+0x18,bytes([int(enabled[name])]))
 nations={n['id']:m.alloc(0x80)for n in rows}
 for n in rows:
  p=nations[n['id']];m.w(p+0x48,m.array([factions[f['id']]for f in n['factions']])if n['factions']else 0);m.w(p+0x4c,len(n['factions']));m.u.mem_write(p+0x68,bytes([int(n['selectable'])]))
 world=m.alloc(0x400);data=m.alloc(0x600);m.w(0xb36534,world);m.w(0xb36530,data);m.w(world+0xd4,capture['kingdoms'][0]['seed_before']);m.w(data+0x80,m.array(list(nations.values())));m.w(data+0x84,len(nations))
 a=m.alloc(0x80);s=m.alloc(0x80);m.w(0xb3655c,a);m.w(0xb36560,s)
 def log(mm):assert mm.arg(2)==m.r(world+0xd4);events.append(['A'if mm.ecx()==a else'S',mm.arg(2)]);mm.ret(pop=12)
 m.seam(0x77da70,log)
 def commander(mm):
  k=mm.ecx();m.w(k+0x230,m.array([0,mm.arg(1)])if mm.arg(1)else 0);mm.ret(pop=4)
 m.seam(0x4e0f00,commander)
 results=[]
 for expected in capture['kingdoms']:
  assert m.r(world+0xd4)==expected['seed_before'];k=m.alloc(0x400);m.w(k+0x230,0);m.w(k+0x234,0);m.w(k+0x2e0,0)
  m.u.reg_write(UC_X86_REG_ESI,k);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESP,nm.STACK);m.u.emu_start(0x50ec66,0x50ecfa,count=100000)
  assert m.u.reg_read(UC_X86_REG_EIP)==0x50ecfa and m.u.reg_read(UC_X86_REG_ESP)==nm.STACK
  n=next(name for name,p in nations.items()if p==m.r(k+0x234));cp=m.r(k+0x230);f=next((name for name,p in factions.items()if cp and p==m.r(cp+4)),None)
  result=dict(id=expected['id'],seed_before=expected['seed_before'],seed_after=m.r(world+0xd4),nation=n,commander=f);assert result==expected,(result,expected);results.append(result)
 assert sum(e[0]=='A'for e in events)+1==capture['logsA'];assert sum(e[0]=='S'for e in events)+1==capture['logsS']
 return dict(kingdoms=results,logs=events),{a:b for a,b in m.visited.items()if a not in {0x6f7810,0x6f7430,0x77da70,0x4e0f00}}
def main():
 c=HERE/'actual-source-input.json';capture=json.loads(c.read_text());rows=[];pins={}
 for pc in nm.PRECISIONS:
  a,pa=run(capture,'original',pc);b,pb=run(capture,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,output=a))
 for p,b in pins.items():assert nm.original.range(p,len(b))==nm.modified.range(p,len(b))==b
 doc=dict(scope=__doc__,capture_sha256=hashlib.sha256(c.read_bytes()).hexdigest(),rows=rows,native_kingdom_executions=6*len(capture['kingdoms']),instructions=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(pins.items())]);p=HERE/'native-actual-configure-nations.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(kingdoms=len(capture['kingdoms']),native_kingdom_executions=doc['native_kingdom_executions'],pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()

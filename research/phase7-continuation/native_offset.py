"""Whole original52FD90+51D510+445400/CRT+FPTAN, no decision hooks."""
from pathlib import Path
import sys,json,copy,hashlib,random,math,struct
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sys.path.insert(0,str(ROOT/'research/phase5-navigation'))
import native_oracle as base
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def main():
 pe=base.Pe(ROOT.parent/'AA.exe',base.ORIGINAL_SHA);v14=base.Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',base.ACCEPTED_SHA)
 rng=random.Random(0x52fd90); rows=[]; visited={}
 samples=[('zero_radius',0,[base.node(),base.node(20,3,0,1)]),('negative_zero_radius',0x80000000,[base.node(),base.node(20,3,0,1)]),('empty',base.bits(2),[]),('one',base.bits(2),[base.node()]),('kindzero',base.bits(2),[base.node(kind=0),base.node(20,3,0,1)]),('equal_xy_two',base.bits(2),[base.node(),base.node()]),('equal_xy_three',base.bits(2),[base.node(),base.node(),base.node(20,3,0,1)]),('near_xy_three',base.bits(2),[base.node(),base.node(10.00001,2.00001),base.node(20,3,0,-1)])]
 for k in range(50):
  x=rng.uniform(-3.1,3.1);y=rng.uniform(-3.1,3.1)
  samples.append((f'turn{k}',base.bits(rng.uniform(.001,50)),[base.node(0,0,math.cos(x),math.sin(x)),base.node(5,8,math.cos(y),math.sin(y))]))
 for pc in ('Single24','Double53','Extended64'):
  for pending in (False,True):
   for name,radius,nodes in samples:
    c=copy.deepcopy(base.cases()[0]);c['precision']=pc;c['state']['path']=base.path(nodes)
    m=base.Machine(pe,c);m.put(base.MOVER+0x30,radius);m.u.reg_write(UC_X86_REG_FPSW,0x20 if pending else 0)
    m.u.hook_add(UC_HOOK_CODE,lambda u,at,size,_:visited.setdefault(at,size) if at<0x1000000 else None)
    stack=0x3008000;m.put(stack,base.STOP);m.u.reg_write(UC_X86_REG_ESP,stack);m.u.reg_write(UC_X86_REG_ECX,base.STATE)
    regs=[(UC_X86_REG_EBX,0x11223344),(UC_X86_REG_ESI,0x33445566),(UC_X86_REG_EDI,0x55667788),(UC_X86_REG_EBP,0x778899aa)]
    for r,v in regs:m.u.reg_write(r,v)
    m.u.emu_start(0x52fd90,base.STOP,count=200000)
    assert m.u.reg_read(UC_X86_REG_EIP)==base.STOP and m.u.reg_read(UC_X86_REG_ESP)==stack+4
    assert all(m.u.reg_read(r)==v for r,v in regs)
    assert not m.events
    rows.append(dict(name=name,precision=pc,pending=pending,radius=radius,state=c['state'],output=m.ints(base.STATE+0x54)[0]))
 for at,size in visited.items(): assert pe.at(at,size)==v14.at(at,size)
 report=dict(schema=1,passed=True,producer_sha256=sha(Path(__file__)),original_sha256=base.ORIGINAL_SHA,v14_sha256=base.ACCEPTED_SHA,cases=rows,visited=[dict(address=hex(at),size=size,bytes=pe.at(at,size).hex()) for at,size in sorted(visited.items())],limits=['Finite normal gameplay directions; x87 hardware comparison provided by Rust tests. Incoming PE explicit.'])
 (OUT/'native-offset.json').write_text(json.dumps(report,indent=2)+'\n');print(len(rows))
if __name__=='__main__':main()

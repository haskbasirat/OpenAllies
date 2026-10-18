"""Execute original NiTimeController clock over ordered frame/rate changes.
No clock/application services are stubbed: bit0 external application-time mode is
excluded. The caller's f32 result-store boundary is explicit. Global timing cache
is initialized with unmatched mode then retained across the sequence.
"""
from pathlib import Path
import json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn.x86_const import *
F=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
RANGES={'keyframe_update':(0x8c0200,0x8c0236),'keyframe_constructor':(0x8c0370,0x8c0391),'controller_reset':(0x89e990,0x89e9ab),'scaled_time':(0x89e9b0,0x89eb92),'sequence_input':(0x883f50,0x883f83),'assignment':(0x717276,0x717343),'vtable':(0xabe1a0,0xabe1f0),'max_time':(0xab75f4,0xab75f8)}
for a,b in RANGES.values():assert original.range(a,b-a)==modified.range(a,b-a)
assert struct.unpack('<I',original.range(0xab75f4,4))[0]==0x7f7fffff
rows=[]
for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
 for mode in [0,1,2,3]:
  for reverse in [False,True]:
   for start,stop in [(0.,1.),(.2,.9),(2.,2.)]:
    m=Machine(cw);m.reset(0);m.w(m.STATE+0x20,0xff7fffff);m.w(m.STATE+0x24,0xff7fffff);m.w(m.STATE+0x28,F(7));m.w(0xb32454,0xffffffff)
    steps=[]
    for index,time in enumerate([0.,.01,.0333333333,.1,.5,.8,1.,1.1,2.,3.5,5.25,4.,-0.3,0.,.2,.4]):
     frequency=[1.,.3,2.,.7][(index//4)%4];phase=[0.,.13][(index//6)%2];flags=32|(mode<<1)|(16 if reverse else 0)
     m.u.context_restore(m.initial);m.u.reg_write(UC_X86_REG_ECX,m.STATE);m.u.reg_write(UC_X86_REG_ESP,m.STACK)
     m.w(m.STACK,m.STOP-16);m.w(m.STACK+4,F(time));m.w(m.STACK+8,m.STOP)
     m.u.mem_write(m.STOP-16,b'\xd9\x1d'+struct.pack('<I',m.STATE+0x200)+b'\xc3')
     for o,v in [(0xc,flags),(0x10,F(frequency)),(0x14,F(phase)),(0x18,F(start)),(0x1c,F(stop))]:m.w(m.STATE+o,v)
     m.u.emu_start(0x89e9b0,m.STOP,count=10000)
     steps.append(dict(input=F(time),controller=dict(flags=flags,frequency=F(frequency),phase=F(phase),start=F(start),stop=F(stop)),output=m.r(m.STATE+0x200),state=dict(first=m.r(m.STATE+0x20),last=m.r(m.STATE+0x24),accumulated=m.r(m.STATE+0x28))))
    rows.append(dict(precision=precision,steps=steps))
out={'format':'aa-native-controller-clock-v1','original_sha256':sha(original.b),'windows':{k:{'start':hex(a),'end':hex(b),'sha256':sha(original.range(a,b-a))}for k,(a,b)in RANGES.items()},'cases':rows,'seams':['Native89E9B0 and internal96DB7A remainder execute unmodified','Finite external-time inputs; application-time bit0 excluded','Caller f32 store modeled by small return stub','Original global cache starts modeFFFFFFFF then remains live; not process-global initialization claim']}
(HERE/'native-controller-clock.json').write_text(json.dumps(out,indent=2)+'\n');print(len(rows),sum(len(r['steps'])for r in rows))

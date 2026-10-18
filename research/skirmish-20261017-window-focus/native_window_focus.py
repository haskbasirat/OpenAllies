"""Original/v14 application flags word prefixes. No original writes."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified,PRECISIONS
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESI,UC_X86_REG_EBX
from unicorn import UC_HOOK_CODE

def image(m,pe):
    if pe is modified:
        old=(original.image_size+0xfff)&~0xfff
        new=(modified.image_size+0xfff)&~0xfff
        if new>old:m.u.mem_map(original.base+old,new-old)
        for vs,rva,raw,off in modified.sections:m.u.mem_write(modified.base+rva,modified.b[off:off+raw])
        m.u.hook_add(UC_HOOK_CODE,lambda u,a,n,_:m.visited.__setitem__(a,bytes(u.mem_read(a,n))),begin=original.base+old,end=modified.base+new-1)

def message(pe,pc,msg,wparam):
    m=Machine(pc);image(m,pe);calls=[]
    m.seam(0x40B000,lambda mm:mm.ret_st0_f32(0))
    m.seam(0x404F60,lambda mm:mm.ret())
    for entry,kind in [(0x40DAC0,'activate'),(0x40DA30,'deactivate')]:
      def callback(mm,kind=kind):calls.append([kind,mm.arg(1)]);mm.ret(pop=4)
      m.seam(entry,callback)
    m.observe(0x6FDAAE,lambda mm:mm.u.emu_stop())
    try:m.call(0x6FD4A0,args=[0x1234,msg,wparam,0],pop=16)
    except AssertionError:assert m.u.reg_read(UC_X86_REG_EIP)==0x6FDAAE
    pins={f'{a:08X}':b.hex() for a,b in m.visited.items() if a!=0x6FDAAE}
    assert all(pe.range(int(a,16),len(bytes.fromhex(b)))==bytes.fromhex(b) for a,b in pins.items())
    return calls,pins

def run(pe,pc,before,activate,mask):
    m=Machine(pc)
    image(m,pe)
    p=m.alloc(0x100);m.w(p+0x34,before)
    cutoff=0x40DAF4 if activate else 0x40DA57
    tail=0x40DB41 if activate else 0x40DABA
    reached=[]
    def stop(mm):reached.append(True);mm.u.emu_stop()
    m.observe(cutoff,stop)
    m.observe(tail,lambda mm:mm.u.emu_stop())
    try:m.call(0x40DAC0 if activate else 0x40DA30,ecx=p,args=[mask],pop=4)
    except AssertionError:
        assert m.u.reg_read(UC_X86_REG_EIP) in [cutoff,tail]
    pins={f'{a:08X}':b.hex() for a,b in m.visited.items() if a not in [cutoff,tail]}
    assert all(pe.range(int(a,16),len(bytes.fromhex(b)))==bytes.fromhex(b) for a,b in pins.items())
    return {'after':m.r(p+0x34),'transition':(1 if activate else 2) if reached else 0},pins

def main():
    rows=[];pins={};v14pins={}
    for pc in PRECISIONS:
      for before in [0,1,2,3,4,0x80000000,0xFFFFFFFF,0xA5A5A5A5]:
       for active in [False,True]:
        for mask in [0,1,2,3,4,6,0x80000000,0xFFFFFFFF]:
         one,p1=run(original,pc,before,active,mask);two,p2=run(modified,pc,before,active,mask)
         assert one==two
         rows.append(dict(pc=pc,before=before,activate=active,mask=mask,**one));pins.update(p1);v14pins.update(p2)
    #Actual constructor-zero instruction; no unsupported whole-constructor claim.
    for pc in PRECISIONS:
     for pe in [original,modified]:
      m=Machine(pc)
      image(m,pe)
      p=m.alloc(0x100);m.w(p+0x34,0xDEADBEEF)
      m.u.reg_write(UC_X86_REG_ESI,p);m.u.reg_write(UC_X86_REG_EBX,0)
      m.u.emu_start(0x41781A,0x41781D)
      assert m.r(p+0x34)==0
      pins['0041781A']=original.range(0x41781A,3).hex()
    messages=[]
    for pc in PRECISIONS:
     for msg,values in [(6,[0,1,2,0x10000,0x10001,0xFFFF0001]),(0x1C,[0,1,0xFFFFFFFF]),(7,[0,1])]:
      for value in values:
       a,p1=message(original,pc,msg,value);b,p2=message(modified,pc,msg,value);assert a==b
       messages.append(dict(pc=pc,message=msg,wparam=value,calls=a));pins.update(p1);v14pins.update(p2)
    out={'scope':'actual word producer prefixes only; transition effects cut before diagnostic, observed cutoff/tail excluded from executed pins; v14 real40DA30 wrapper/trampoline prefix executes but post-return custom input clearing excluded; WndProc message routing uses explicit timer/log/activation transport seams','original_sha256':hashlib.sha256(original.b).hexdigest(),'v14_sha256':hashlib.sha256(modified.b).hexdigest(),'executions':len(rows)*2+6+len(messages)*2,'rows':rows,'messages':messages,'executed_pins':pins,'v14_executed_pins':v14pins}
    (HERE/'native-window-focus.json').write_text(json.dumps(out,indent=2)+'\n')
    #One copy per numeric input; same outputs proven at every PC/image.
    selected=[r for r in rows if r['pc']=='Double53']
    (HERE/'native-focus-rows.rs').write_text('&[\n'+''.join(f"({r['before']}, {str(r['activate']).lower()}, {r['mask']}, {r['after']}, {r['transition']}),\n" for r in selected)+']\n')
    print(json.dumps({'rows':len(rows),'executions':out['executions'],'pins':len(pins)}))
if __name__=='__main__':main()

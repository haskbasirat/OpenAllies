"""Native constructor MOV projection + whole751770 fresh Reset, and actual
74D5F0 attachment/74B310 detach. No whole75CE80 ctor/startup certificate.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE.parent));sys.path.insert(0,str(HERE.parent/'host'))
from native_local_host import HostMachine,PE0,PE1,PC,NODE,WRITER_VT,proof_ranges
from unicorn.x86_const import *

def fields(m):
 return {k:(m.rb(NODE+off)if byte else m.r(NODE+off))for k,off,byte in [
  ('running',0x2c0,True),('admin_time',0x28c,False),('disconnecting',0x2cd,True),
  ('disconnect_started',0x2d0,False),('heartbeat',0x2dc,False),('recorder',0x268,False),('playback',0x26c,False)]}
def main():
 rows=[];visited={}
 stores=[(0x75d074,6),(0x75d07a,6),(0x75d0db,6),(0x75d172,6),(0x75d190,6),(0x75d196,6),(0x75d1a8,6)]
 for image in ('original','v14'):
  for p in PC:
   for poison in (0xa5a5a5a5,0xffffffff):
    m=HostMachine(image);m.fpu(p);m.begin(0x12345678,list(range(16)),0)
    for off in (0x268,0x26c,0x28c,0x2c0,0x2cd,0x2d0,0x2dc):m.w(NODE+off,poison)
    m.u.reg_write(UC_X86_REG_ESI,NODE);m.u.emu_start(0x75ceb2,0x75ceb4,count=1)
    for va,n in stores:m.u.emu_start(va,va+n,count=1)
    ctor=fields(m);assert set(ctor.values())=={0}
    for off in (0x1f8,0x254,0x270):
     x=m.stream(WRITER_VT,size=64);m.u.mem_write(NODE+off,bytes(m.u.mem_read(x,16)))
    m.call(0x74b790,[0x100],this=NODE+0x224,callee_pop=4);m.w(NODE+0x64,0);m.w(NODE+0x68,0)
    for off in (0x28c,0x2c0,0x2cd,0x2d0,0x2dc):m.w(NODE+off,poison)
    m.call(0x751770,this=NODE);reset=fields(m)
    assert reset==dict(ctor,disconnect_started=poison)
    # Non-owned attachment pointer is an opaque allocated token, never used
    # as a recorder vtable. The function itself only stores that pointer.
    token=m.alloc(4);m.w(NODE+0x250,0x12345678);m.w(NODE+0x25c,0x11223344);m.w(NODE+0x260,0x55667788)
    buf=m.r(NODE+0x274);m.u.mem_write(buf,b'\xff'*64);m.w(NODE+0x278,19);m.w(NODE+0x27c,6)
    m.call(0x74d5f0,[token],this=NODE,callee_pop=4)
    attach={'attached':m.r(NODE+0x268)==token,'mirror_bytes':list(m.u.mem_read(buf,64)),'byte':m.r(NODE+0x278),'bit':m.r(NODE+0x27c),
      'field250':m.r(NODE+0x250),'resume_byte':m.r(NODE+0x25c),'resume_bit':m.r(NODE+0x260)}
    assert attach['attached'] and attach['mirror_bytes']==[0]*64 and attach['byte']==0 and attach['bit']==3
    assert [attach[k]for k in ('field250','resume_byte','resume_bit')]==[0x12345678,0x11223344,0x55667788]
    m.u.mem_write(buf,b'\x91'*64);m.w(NODE+0x278,5);m.w(NODE+0x27c,7);m.call(0x74b310,this=NODE)
    detach={'attached':m.r(NODE+0x268)!=0,'mirror_bytes':list(m.u.mem_read(buf,64)),'byte':m.r(NODE+0x278),'bit':m.r(NODE+0x27c)}
    assert not detach['attached'] and detach['mirror_bytes']==[0x91]*64 and detach['byte']==5 and detach['bit']==7
    rows.append({'image':image,'precision':p,'poison':poison,'constructor':ctor,'reset':reset,'attach':attach,'detach':detach});visited.update(m.visited)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 out={'passed':True,'scope':'Constructor XOR/MOV projection; whole fresh751770 reset; whole74D5F0/74B310. Successful named Reset dependencies only; not whole ctor.',
 'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':hashlib.sha256(PE0.b).hexdigest(),'v14_sha256':hashlib.sha256(PE1.b).hexdigest(),'cases':rows,'executed_windows':proof_ranges(visited)}
 (HERE/'native-node-frame-fields.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(rows),'node frame/recorder rows')
if __name__=='__main__':main()

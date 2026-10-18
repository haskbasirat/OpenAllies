"""Native Team4E68C0 and4EFA40(NULL), independently loaded original/v14.

Actual Team vtable+C=4E3E90, name copy4BEDF0, logging caller and reference
release run. Required snap46EE00, placement4EF6A0, spatial4EE4C0, terrain
4F0E50 and diagnostic404F60 are explicit service seams.4012C0 supplies
CString reference transport. Failure probes stop inside those explicit
services, not a claimed native throw/unwind or completed spatial algorithm.
"""
from pathlib import Path
import sys,json,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified,sha,instructions
from unicorn import UC_HOOK_MEM_WRITE

class Dependency(Exception):pass
def text(m,ptr):
    units=[]
    for i in range(m.r(ptr-12)):
        units.append(struct.unpack('<H',m.u.mem_read(ptr+i*2,2))[0])
    return units
def run(c,image_name,pc):
    m=Machine(pc);image=original if image_name=='original' else modified
    old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
    if new>old:m.u.mem_map(image.base+old,new-old)
    for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
    obj=m.alloc(0x100);world=m.alloc(0x100);terrain=m.alloc(0x100)
    m.w(obj,0xa29780);m.w(obj+8,c['flags']);m.w(0xb36534,world);m.w(world+0x30,terrain);m.w(terrain+8,c['tile'])
    for i,b in enumerate(c['position']):m.w(obj+0x20+i*4,b)
    data=m.alloc(32+2*len(c['name']))+16
    m.w(data-12,len(c['name']));m.w(data-4,1)
    m.u.mem_write(data,struct.pack('<'+'H'*len(c['name']),*c['name'])+b'\0\0');m.w(obj+0x18,data)
    events=[];stores=[]
    def fail(name):
        if c.get('failure')==name:raise Dependency(name)
    def position():return [m.r(obj+0x20+i*4) for i in range(3)]
    def snap(_):
        assert m.ecx()==0xb3a8c4 and m.arg(1)==obj+0x20 and m.arg(2)==c['tile']
        events.append(['snap',c['tile']])
        for i,b in enumerate(c['snapped']):m.w(obj+0x20+i*4,b)
        fail('snap');m.ret(pop=8)
    def placement(_):
        assert m.ecx()==obj and m.arg(1)==0
        events.append(['placement',position()]);fail('placement');m.ret(c['placement_return'],pop=4)
    def index(_):
        assert m.ecx()==obj
        events.append(['index'])
        if 'index_flags'in c:m.w(obj+8,c['index_flags'])
        fail('index');m.ret()
    def height(_):
        assert m.ecx()==world and m.arg(1)==obj+0x20
        events.append(['height',position(),m.arg(2)]);fail('height')
        m.ret_st0_f64(c['height_f64_bits'],pop=8)
    def cstring(_):
        header=m.arg(1);m.w(header+12,m.r(header+12)+1);m.ret(header)
    def log(_):
        assert m.arg(3)==0xee5
        events.append(['log',text(m,m.arg(6))]);fail('log');m.ret()
    def writes(u,access,address,size,value,user):
        if address==obj+0x28:stores.append(value&0xffffffff)
    for address,callback in [(0x46ee00,snap),(0x4ef6a0,placement),(0x4ee4c0,index),
                             (0x4f0e50,height),(0x4012c0,cstring),(0x404f60,log)]:m.seam(address,callback)
    m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
    outcome='ok';result=None
    try:result=bool(m.call(0x4e68c0,obj,[0])&255)
    except Dependency as error:outcome=str(error)
    m.verify_v14()
    return {'outcome':outcome,'result':result,'flags':m.r(obj+8),'position':position(),
            'name':text(m,m.r(obj+0x18)),'events':events,'height_stores':stores},m.visited

def cases():
    base={'name':[ord(c) for c in 'saved-team-name'],'position':[0x3f800000,0x40000000,0x40400000],
        'snapped':[0x40800000,0x40a00000],'tile':0x40000000,
        'placement_return':1,'height_f64_bits':0x3ff0000010000000,'flags':0}
    for flags in [0,1<<12,1<<21,(1<<21)|(1<<12),1<<16,(1<<16)|(1<<12),(1<<16)|(1<<21)]:
        for result in [0,256,1,255]:yield dict(base,label='flags_'+str(flags)+'_return_'+str(result),flags=flags,placement_return=result)
    for name in ['snap','placement','index','height','log']:
        yield dict(base,label='missing_'+name,flags=1<<16,failure=name)
    for flags in [0,1<<12,1<<21,(1<<21)|(1<<12)]:
        yield dict(base,label='index_changes_flags_'+str(flags),flags=1<<12,index_flags=flags)
    for bits in [0x8000000000000000,0x3ff0000030000000,0x0000000000000001]:
        yield dict(base,label='height_'+hex(bits),height_f64_bits=bits)

def main():
    rows=[];visited={}
    for image in ['original','v14']:
        for pc in ['Single24','Double53','Extended64']:
            for c in cases():
                out,code=run(c,image,pc);visited.update(code)
                rows.append({'image':image,'precision':pc,'input':c,'output':out})
    out={'schema':1,'scope':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),
        'producer_sha256':sha(Path(__file__).read_bytes()),'harness_sha256':sha((ROOT/'research/phase8-ai/native_machine.py').read_bytes()),
        'null_parameter_only':True,'cases':rows,'instructions':instructions(visited)}
    path=HERE/'native-team-init.json'
    if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'native Team Init evidence differs'
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes()),'instructions':len(visited)}))
if __name__=='__main__':main()

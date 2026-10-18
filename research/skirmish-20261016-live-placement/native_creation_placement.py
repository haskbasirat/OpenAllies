"""Untouched 4F3640 coordinator, constructor/snap/outside geometry at all PCs.
Explicit service probes cover required-zone/footprint validation and zone lists.
Both original and v14 are separately mapped; no SDK source is used.
"""
from pathlib import Path
import copy, hashlib, json, sys
HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT/'research/phase8-movement'))
import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32
ORIGINAL = h.P
H = lambda b: hashlib.sha256(b).hexdigest()

class Required(Exception): pass

def run(c, image, pc):
    h.P = ORIGINAL if image == 'original' else h.V
    m = h.Machine(pc)
    world=m.alloc(0x600); settings=m.alloc(0x200); map_=m.alloc(0x100); tiles=m.alloc(0x180)
    d=m.alloc(0x600); otherd=m.alloc(0x600); zoned=m.alloc(0x600)
    other=m.alloc(0x200); source=m.alloc(0x200); za=m.alloc(0x200); zone=m.alloc(0x80); gacd=m.alloc(0x54)
    ids={d:1,otherd:2,zoned:3}; actors={other:2,source:1,za:3}
    m.w32(0xb36534,world);m.w32(world+0x28,settings if c['settings'] is not None else 0)
    m.w32(world+0x30,map_);m.w32(settings+0xa8,c['settings'] or 0)
    m.w32(map_+4,2,h.f2b(4),*c['extents']);m.w32(map_+0x1c,8,8);m.w32(map_+0x28,tiles)
    m.w32(gacd+4,d);m.w32(gacd+8,*c['position']);m.w32(gacd+0x14,*c['facing'])
    for o,v in [(0x28,c['owner']),(0x38,c['mode38']),(0x3c,c['explicit'])]:m.w32(gacd+o,v)
    m.w8(gacd+0x52,c['byte52']);m.w32(other+4,otherd);m.w32(other+8,c['other_flags'])
    m.w32(source+4,otherd);m.w32(za+4,zoned);m.w32(zone+4,za)
    category=m.alloc(0x20);m.w32(zoned+0x18,category);m.w32(category+0xc,c['zone_category'])
    for o,v in [(0x1c,c['flags']),(0x174,c['mask174']),(0x430,c['word430']),(0x444,c['word444']),
                (0x4e0,otherd if c['required'] else 0)]:m.w32(d+o,v)
    m.w8(d+0x1a8,c['byte1a8']);m.w8(d+0x290,c['byte290'])
    m.w32(d+0x38,*c['shape']);m.w32(otherd+0x1c,0)
    events=[]; context=None; iterator=None; cursor=0
    def event(name,*args):
        events.append([name,*args])
        if c.get('failure')==name:raise Required(name)
    def stub(fn):
        a=m.stub_next;m.stub_next+=16;m.u.mem_write(a,b'\xc3');m.seams[a]=fn;return a
    def virt(which,slot):
        def f(_):
            event('virtual',which,slot)
            key={0x30:'object_data',0x38:'actor_data',0x2c:'v2c',0x3c:'v3c'}[slot]
            value=c[key] if which==1 else c['other_v2c'] if which==2 else c['zone_v3c']
            m.ret(0,value)
        return f
    for p,n in [(d,1),(otherd,2),(zoned,3)]:
        vt=m.alloc(0x80);m.w32(p,vt)
        for slot in [0x30,0x38,0x2c,0x3c]:m.w32(vt+slot,stub(virt(n,slot)))
    def virtual_shape(_):
        event('virtual_shape');out=m.arg();m.w32(out,*c['virtual_shape']);m.ret(4,out)
    m.w32(m.r32(d)+0x28,stub(virtual_shape))
    def validation(_):
        event('required', [m.arg(0),m.arg(1)],ids[m.arg(2)],m.arg(3),actors.get(m.arg(4),0),m.arg(5),m.arg(6))
        m.ret(28,c['validation'])
    m.seams[0x4f1660]=validation
    def begin(_):
        nonlocal iterator,cursor
        iterator=m.ecx();cursor=0
        placement=m.arg(1);event('begin',[m.r32(placement+12),m.r32(placement+16)])
        # Native iterator offsets: map+0, maxY+10, currentX+24/currentY+28.
        m.w32(iterator,map_);m.w32(iterator+0x10,0);m.w32(iterator+0x24,0);m.w32(iterator+0x28,0 if c['zones'] else 1)
        m.ret(16,iterator)
    def next_(_):
        nonlocal cursor
        event('advance');cursor+=1
        m.w32(iterator+0x24,cursor);m.w32(iterator+0x28,0 if cursor<len(c['zones']) else 1);m.ret(4,0)
    def zone_(_):
        event('tile_zone',cursor);m.ret(12,zone if c['zones'][cursor] else 0)
    def overlap(_):
        event('zone_overlap',actors[m.arg(0)]);m.ret(8,c['overlap'])
    m.seams[0x554dd0]=begin;m.seams[0x554330]=next_;m.seams[0x54ef80]=zone_;m.seams[0x4f0ae0]=overlap
    def structure(_):event('structure',[m.arg(0),m.arg(1)]);m.ret(8,za if c['structure'] else 0)
    m.seams[0x4f2220]=structure
    def collision(_):
        nonlocal context
        q=m.arg(0)
        context={'definition':ids[m.r32(q+0x18)],'source_actor':actors.get(m.r32(q+0x1c),0),
                 'ignore_actor':actors.get(m.r32(q+0x20),0),'ignore_definition':ids.get(m.r32(q+0x24),0),
                 'owner':m.r32(q+0x2c),'custom_filter':m.r32(q+0x34),'ignore_all':bool(m.r8(q+0x30)),'ignore_moving':bool(m.r8(q+0x31)),
                 'required':m.r32(q+0x38),'mask':m.r32(q+0x3c),
                 'shape':m.r32(q,6),'facing':m.r32(q+0x44,2),'position':m.r32(q+0x4c,2)}
        event('collision',context);m.ret(8,c['collision'])
    m.seams[0x4f0a70]=collision
    def footprint(_):event('footprint');m.w32(m.arg(1),0x13579bdf);m.ret(8,c['footprint'])
    m.seams[0x4f0b70]=footprint
    def observe(u,address,size,_):
        if address==0x4f3727:event('shape',u.reg_read(UC_X86_REG_EAX)==d+0x38)
    m.u.hook_add(UC_HOOK_CODE,observe)
    result=None;outcome='ok'
    try:
        result=m.call(0x4f3640,world,[gacd,source if c['source'] else 0,other if c['other'] else 0,
                       c['arg5'],c['arg6'],c['arg7'],za if c['last']==3 else other if c['last']==2 else 0])
        assert m.final_esp==h.STACK+32
        assert m.fpu_top()==0
    except Required as e:outcome=str(e)
    return {'result':result,'outcome':outcome,'events':events,'context':context}

def cases():
    b=dict(object_data=1,actor_data=0,v2c=0,v3c=0,other_v2c=0,zone_v3c=0,
           position=[h.f2b(10.3),h.f2b(12.8)],facing=[0,0xbf800000],extents=[h.f2b(32)]*2,
           flags=0,mask174=0,word430=0,word444=0,required=False,byte1a8=0,byte290=0,
           owner=0x12345678,mode38=0,explicit=0,byte52=0,settings=0,other_flags=0,
           source=True,other=True,last=0,arg5=0,arg6=0,arg7=0x87654321,
           shape=[1,h.f2b(1),0,0,0,0],virtual_shape=[1,h.f2b(2),0,0,0,0],
           zones=[],zone_category=0,overlap=1,structure=False,validation=0,collision=0,footprint=4)
    yield dict(b,label='ordinary')
    for k,values in [('flags',[1<<10,0x20000400,0x20010400,1<<18,1<<21]),('settings',[None,1,2]),
                     ('other_flags',[0x80000000,1<<12]),('other_v2c',[1,0x100]),('v2c',[1,0x100]),
                     ('collision',[1,2,3,4,0xffffffff]),('validation',[1,2,3,4,5]),('arg5',[1,0x100,0xffffffff]),
                     ('arg6',[1,0x100]),('mode38',[3]),('byte52',[1]),('explicit',[1]),
                     ('last',[2,3]),('object_data',[0,0x100]),('actor_data',[1,0x100])]:
        for v in values:yield dict(b,label=f'{k}_{v}',**{k:v})
    for x in [0,0x80000000,h.f2b(31),h.f2b(32),h.f2b(-1)]:yield dict(b,label=f'position_{x}',position=[x,h.f2b(10)])
    for flag in [0x20000400,0x20000c00]:
        for strict in [False,True]:
            for ad in [0,1]:
                for structure in [False,True]:
                    for arg6 in [0,1]:
                        yield dict(b,label=f'zone_{flag}_{strict}_{ad}_{structure}_{arg6}',flags=flag,
                                   other=strict,actor_data=ad,zones=[False,True],structure=structure,arg6=arg6)
    for field,val in [('word430',1),('mask174',1<<12),('mask174',1<<13),('byte1a8',1),('byte290',1)]:
        for structure in [False,True]:
            c=dict(b,label=f'zone_allow_{field}_{val}_{structure}',flags=0x20000c00,actor_data=1,
                   other=False,zones=[True],structure=structure,word444=1)
            c[field]=val;yield c
    for req in [False,True]:
        for arg6 in [0,1]:
            for fp in [0,1,2,3,4,5]:
                yield dict(b,label=f'required_object_{req}_{arg6}_{fp}',actor_data=1,word444=1,
                           other=False,arg6=arg6,required=req,footprint=fp)
    # Only reached required callbacks become service-failure prefixes.
    prefixes=[dict(b,label='failure_base'),dict(b,label='failure_zone',flags=0x20000400,zones=[False,True]),
              dict(b,label='failure_required',v2c=1),dict(b,label='failure_footprint',actor_data=1,other=False,word444=1)]
    seen=set()
    for c in prefixes:
        for e in run(c,'original','Double53')['events']:
            name=e[0]
            if name in seen:continue
            seen.add(name);yield dict(c,label='failure_'+name,failure=name)

def main():
    rows=[]
    inputs=list(cases())
    for pc in h.PRECISION:
        for c in inputs:
            a=run(c,'original',pc);b=run(c,'v14',pc);assert a==b,(pc,c['label'],a,b)
            rows.append(dict(precision=pc,input=c,output=a))
    cs=Cs(CS_ARCH_X86,CS_MODE_32)
    result=dict(schema=1,scope=__doc__,original_sha256=H(ORIGINAL.b),v14_sha256=H(h.V.b),
                native_executions=2*len(rows),cases=rows,
                instructions=[dict(address=hex(a),bytes=b,asm='; '.join(i.mnemonic+' '+i.op_str for i in cs.disasm(bytes.fromhex(b),a))) for a,b in sorted(h.VISITED.items())],
                seams=['Definition virtual+28/+2C/+30/+38/+3C: explicit probes, not source proof',
                       '4F1660,4F0AE0,4F2220,4F0A70,4F0B70: mandatory probes; actual child bodies excluded',
                       '554DD0/554330/54EF80: supplied ordered fixture tile/zone stream',
                       '46E0A0/553090/552250/553370 and reached numeric leaves execute untouched'])
    path=HERE/'native-creation-placement.json'
    if '--write' in sys.argv:path.write_text(json.dumps(result,indent=1)+'\n')
    else:
        old=json.loads(path.read_text());assert old==result,'fixture differs; no files overwritten'
    print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(result['instructions']),sha256=H(path.read_bytes()))))
if __name__=='__main__':main()

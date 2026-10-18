"""Original/v14 4D7230 with a real NULL load parameter.

The coordinator, GTID checks, definition+24, TrackInfo getters, CRT floor,
_ftol and scalar arithmetic execute natively. Object4EBE50, required-object
4D1FD0, component methods and world subsystems are explicit required leaves.
Leaf mutation/failure probes describe ordering, not normal gameplay effects.
No complete Object/world implementation or native SEH-unwind claim is made.
"""
from pathlib import Path
import sys,json,copy,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import Machine,original,modified,sha,instructions
from unicorn import UcError
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_EIP
class Dependency(Exception):pass
def run(c,image_name,pc):
    m=Machine(pc);image=original if image_name=='original' else modified
    old=(original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
    if new>old:m.u.mem_map(image.base+old,new-old)
    for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
    a=m.alloc(0x200);idmgr=m.alloc(0x60010);m.w(0xb364c0,idmgr)
    gd=m.alloc(0x500);globals_=m.alloc(0x100);gprop=m.alloc(4)
    m.w(0xb36530,gd);m.w(gd+0x464,globals_);m.w(globals_+0x24,gprop);m.w(gprop,c['property'])
    cache=m.alloc(0x100);scale=m.alloc(4);divisor=m.alloc(4)
    m.w(0xb39ab0,cache);m.w(cache+0x1c,scale);m.w(cache+0x28,divisor)
    m.w(scale,c['scale']);m.w(divisor,c['divisor'])
    sai=m.alloc(4);m.w(0xb36528,sai if c['sai'] else 0)
    req=m.alloc(0x200);vt=m.alloc(0x140);m.w(req,vt)
    m.w(idmgr+0x20004+2*4,req if c['required_live'] else 0)
    m.w(idmgr+0x20004+3*4,req if c['associated_live'] else 0)
    m.w(a+0x158,c['required']);m.w(a+0x50,c['associated'])
    m.w(a+8,c['flags']);m.w(a+0x100,c['state']);m.w(a+0x138,c['word138']);m.w(a+0x144,c['word144'])
    for i,b in enumerate(c['xy']):m.w(a+0x20+i*4,b)
    zone=m.alloc(0x40);zones=m.alloc(12);m.w(zones+8,zone if c['zone_present'] else 0)
    m.w(a+0x148,zones);m.w(a+0x14c,c['zone_count'])
    manager=m.alloc(0x100);m.w(0xb39f58,manager)
    definitions={};track_identity={};events=[];count_reads=[0]
    for identity in [1,2]:
        d=m.alloc(0x300);t=m.alloc(max(1,len(c['tracks']))*0x24)
        m.w(d,0xa05540);m.w(d+0x25c,t);m.w(d+0x260,len(c['tracks']));m.wb(d+0x34,c['needs_zone'])
        definitions[identity]=d
        for i,row in enumerate(c['tracks']):
            m.w(t+i*0x24,row[0]);m.w(t+i*0x24+0x20,row[1]);track_identity[t+i*0x24]=[identity,i]
    m.w(a+4,definitions[1]);def_id=lambda ptr:next(k for k,v in definitions.items()if v==ptr)
    components={};alloc=m.alloc(0x20)
    def mutate(name):
        for change in c.get('mutations',[]):
            if change['at']!=name:continue
            for slot in change.get('add_slots',[]):m.w(a+0x60+slot*4,components[slot])
            for slot in change.get('remove_slots',[]):m.w(a+0x60+slot*4,0)
            for key,offset in [('flags',8),('state',0x100),('required',0x158),('associated',0x50),('word138',0x138),('word144',0x144)]:
                if key in change:m.w(a+offset,change[key])
            if 'definition'in change:m.w(a+4,definitions[change['definition']])
    def event(name,*args):
        events.append([name,*args]);mutate(name)
        if c.get('failure')==name:raise Dependency(name)
    for slot in range(31):
        p=m.alloc(0x30);v=m.alloc(0x20);m.w(p,v);m.w(p+4,a);components[slot]=p
        def init(_,slot=slot):
            event('init_'+str(slot));m.ret(c.get('component_returns',{}).get(str(slot),1))
        def after(_,slot=slot):event('after_'+str(slot));m.ret()
        m.w(v+8,m.stub(init));m.w(v+0x14,m.stub(after))
        if slot in c['slots']:m.w(a+0x60+slot*4,p)
    def object_(_):
        assert m.ecx()==a and m.arg(1)==0;event('object_null');m.ret(c['object_return'],pop=4)
    def required_(_):event('required_init');m.ret(c['required_return'])
    def attach(_):
        assert m.ecx()==req and m.arg(1)==a;event('attach',m.r(a+0x158));m.ret(pop=4)
    m.w(vt+0x128,m.stub(attach))
    def simple(name,pop=0,result=0):
        def callback(_):event(name);m.ret(result,pop=pop)
        return callback
    m.seam(0x4ebe50,object_);m.seam(0x4d1fd0,required_)
    for address,name in [(0x4d4cf0,'refresh_fields'),(0x4d1350,'refresh_actions'),(0x4d67f0,'refresh_overlaps'),
                         (0x4d4130,'refresh_zones'),(0x5788b0,'organization'),(0x585650,'settlement'),
                         (0x57e430,'performer'),(0x569e40,'denizen')]:m.seam(address,simple(name))
    def applies(_):
        assert m.arg(1)==a;event('property_applies',m.arg(2));m.ret(c['property_applies'])
    def add_property(_):
        assert m.ecx()==components[6] and m.arg(2)==0 and m.arg(3)==a
        event('property_add',m.arg(1));m.ret(pop=12)
    m.seam(0x5035e0,applies);m.seam(0x5817a0,add_property)
    m.observe(0x4d734a,lambda _:event('global_property',m.r(gprop)))
    m.observe(0x462e50,lambda _:event('needs_zone',def_id(m.ecx())))
    m.seam(0x4d69a0,simple('create_zone2',4,c['create_zone']))
    def radius(_):event('radius');m.ret_st0_f32(c['radius'])
    def move(_):
        if m.ecx()==0:raise Dependency('zone_null')
        event('move_zone',[m.arg(1),m.arg(2)],m.arg(3));m.ret(pop=12)
    m.seam(0x4bb000,radius);m.seam(0x515f40,move)
    m.seam(0x7a0c20,simple('sai',4))
    m.observe(0x4d7450,lambda _:event('track_definition',def_id(m.r(a+4))))
    def track_count(_):
        identity=def_id(m.ecx());event('track_count',identity)
        if count_reads[0]<len(c.get('count_reads',[])):m.w(m.ecx()+0x260,c['count_reads'][count_reads[0]])
        count_reads[0]+=1
    m.observe(0x45e700,track_count)
    m.observe(0x45e710,lambda _:event('track_at',def_id(m.ecx()),m.arg(1)))
    m.observe(0x4d7488,lambda _:event('track_kind',*track_identity[m.u.reg_read(UC_X86_REG_EAX)]))
    for address in [0x4d749b,0x4d74ad]:m.observe(address,lambda _:event('track_length',*track_identity[m.u.reg_read(UC_X86_REG_EAX)]))
    m.observe(0x4d74c0,lambda _:event('scale'))
    m.observe(0x4d74e8,lambda _:event('divisor'))
    def allocate(_):
        assert m.arg(1)==0x20;event('allocate');m.ret(alloc if c['allocate'] else 0)
    def construct(_):
        assert m.ecx()==alloc and m.arg(1)==a
        n=m.arg(2);n=n if n<0x80000000 else n-0x100000000
        event('construct',n);m.ret(alloc,pop=8)
    m.seam(0x6f7810,allocate);m.seam(0x50ab10,construct)
    m.observe(0x4d752c,lambda _:event('track_store',99 if m.u.reg_read(UC_X86_REG_EAX) else None))
    def fx(_):
        assert m.arg(1)==a and m.arg(2)==0
        event('fx',def_id(m.ecx()-0xe4));m.ret(pop=8)
    m.seam(0x42a590,fx)
    outcome='ok';result=None
    try:result=bool(m.call(0x4d7230,a,[0])&255)
    except Dependency as error:outcome=str(error)
    except AssertionError:
        if m.null_accesses:outcome='native_null'
        else:raise
    except UcError:
        if m.u.reg_read(UC_X86_REG_EIP)==0x4d74f4:outcome='native_idiv_fault'
        else:raise
    m.verify_v14()
    w144=m.r(a+0x144)
    return {'outcome':outcome,'result':result,'events':events,'word138':m.r(a+0x138),
        'word144':99 if w144==alloc else w144,'flags':m.r(a+8),'state':m.r(a+0x100),
        'required':m.r(a+0x158),'definition':def_id(m.r(a+4)),
        'slots':[i for i in range(31)if m.r(a+0x60+i*4)]},m.visited

def cases():
    base=dict(label='base',slots=[],object_return=1,required_return=1,flags=0,state=0,word138=0x12345678,word144=77,
        required=0,required_live=True,associated=0,associated_live=False,property=0,property_applies=1,
        needs_zone=False,create_zone=1,zone_present=True,zone_count=3,radius=0x40c00000,xy=[0x3f800000,0x40000000],
        sai=False,tracks=[],scale=0x41200000,divisor=2,allocate=True)
    def make(label,**changes):return dict(copy.deepcopy(base),label=label,**changes)
    yield base
    for key in ['object_return','required_return']:
        for value in [0,256,255]:yield make(key+str(value),**{key:value})
    for slot in range(31):yield make('slot_'+str(slot),slots=[slot])
    yield make('all_slots',slots=list(range(31)))
    for slot in [0,4,30]:yield make('component_false_'+str(slot),slots=list(range(31)),component_returns={str(slot):256})
    for mutations in [
        [{'at':'init_0','add_slots':[4,8],'remove_slots':[3]}],
        [{'at':'init_7','remove_slots':[4]}],
        [{'at':'init_4','add_slots':[0]}],
        [{'at':'after_0','add_slots':[14],'remove_slots':[9],'required':2}],
        [{'at':'refresh_actions','flags':1<<29,'remove_slots':[6]}],
        [{'at':'sai','definition':2}],
        [{'at':'track_definition','definition':2}],
        [{'at':'track_count','definition':2}],
        [{'at':'construct','definition':2}],
        [{'at':'construct','definition':2,'state':1<<21}],
        [{'at':'object_null','word138':0xabcdef01}],
    ]:yield make('mutation_'+str(mutations),slots=[0,3,4,6,7,9],sai=True,tracks=[[1,0x3f800000]],mutations=mutations)
    for required,live in [(2,True),(2,False),(0x10002,True)]:yield make('required_'+str(required)+'_'+str(live),required=required,required_live=live)
    for flags in [0,1<<29]:
        for prop in [0,123]:
            for present in [False,True]:
                for applies in [0,256,1,255]:yield make('property_'+str((flags,prop,present,applies)),flags=flags,property=prop,slots=[6]if present else[],property_applies=applies)
    for needs,assoc,live,created,present,count in [(False,0,False,1,True,3),(True,3,True,1,True,3),
        (True,0,False,0,True,3),(True,0,False,256,True,3),(True,0,False,1,True,3),
        (True,0,False,1,False,3),(True,0,False,1,True,2)]:
        yield make('zone_'+str((needs,assoc,live,created,present,count)),needs_zone=needs,associated=assoc,associated_live=live,create_zone=created,zone_present=present,zone_count=count)
    for sai in [False,True]:
        for flags in [0,1]:
            for state in [0,1<<21]:yield make('sai_fx_'+str((sai,flags,state)),sai=sai,flags=flags,state=state)
    for tracks in [[],[[0,0x7fc00000]],[[1,0]],[[1,0x80000000]],[[1,0xbf800000]],[[1,0x3f800000]],
        [[1,0x3f800001],[0,0x4f000000],[1,0x40000000]],[[1,0x7fc00000]],[[1,1]],[[1,0x3f000001]]]:
        for allocate in [False,True]:yield make('tracks_'+str((tracks,allocate)),tracks=tracks,allocate=allocate)
    for divisor in [0,-1,3]:yield make('divisor_'+str(divisor),tracks=[[0,0]],divisor=divisor)
    yield make('idiv_overflow',tracks=[[1,0x3f800000]],scale=0xcf000000,divisor=-1)
    yield make('count_changes',tracks=[[1,0x40000000],[1,0x40400000]],count_reads=[2,2,1])
    yield make('count_turns_zero',tracks=[[1,0x40000000]],count_reads=[1,0])
    yield make('count_negative',tracks=[[1,0x40000000]],count_reads=[-1])
    for failure in ['object_null','required_init','init_4','after_6','attach','refresh_fields','refresh_actions','refresh_overlaps',
        'global_property','property_applies','property_add','refresh_zones','organization','settlement','performer','denizen',
        'needs_zone','create_zone2','radius','move_zone','sai','track_definition','track_count','track_at','track_kind',
        'track_length','scale','divisor','allocate','construct','track_store','fx']:
        yield make('failure_'+failure,slots=list(range(31)),required=2,property=123,needs_zone=True,sai=True,
            tracks=[[1,0x3f800000]],failure=failure)

def main():
    rows=[];visited={}
    for image in ['original','v14']:
        for pc in ['Single24','Double53','Extended64']:
            for c in cases():
                out,code=run(c,image,pc);visited.update(code)
                rows.append({'image':image,'precision':pc,'input':c,'output':out})
    out={'schema':1,'scope':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),
        'producer_sha256':sha(Path(__file__).read_bytes()),'harness_sha256':sha((ROOT/'research/phase8-ai/native_machine.py').read_bytes()),
        'cases':rows,'instructions':instructions(visited)}
    path=HERE/'native-actor-init-null.json'
    if '--write'in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'native actor Init(NULL) differs'
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes()),'instructions':len(visited)}))
if __name__=='__main__':main()

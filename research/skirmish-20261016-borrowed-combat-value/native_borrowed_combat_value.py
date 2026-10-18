"""Original/v14 5788B0 and 57DC40, all x87 PCs; no arithmetic/call seams.

Named read/store boundaries are instruction hooks for dependency-stop and
mutation probes, not claims that those native loads are virtual callbacks.
Only finite supported arithmetic is exercised. Output stores are captured by
Unicorn memory-write hooks; native instruction bytes and images are pinned.
"""
from pathlib import Path
import copy, json, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT / 'research/phase8-economy'))
from harness import (Machine, ORIGINAL, ACCEPTED, MODES, pin, sha, bits,
    UC_HOOK_CODE, UC_HOOK_MEM_WRITE, UC_X86_REG_EAX, UC_X86_REG_ECX,
    UC_X86_REG_EDX, UC_X86_REG_EDI)

class Dependency(Exception): pass

def run(c, image, cw):
    m = Machine(cw)
    if image == 'v14':
        old_size=(ORIGINAL.size+4095)&~4095
        new_size=(ACCEPTED.size+4095)&~4095
        if new_size>old_size:m.uc.mem_map(ORIGINAL.base+old_size,new_size-old_size)
        for rva, size, offset, _ in ACCEPTED.sections:
            if size: m.uc.mem_write(ACCEPTED.base+rva, ACCEPTED.data[offset:offset+size])
    org = m.alloc(0x100)
    arrays = [m.alloc(0x40) for _ in c['arrays']]
    actors = [m.alloc(0x100) for _ in c['actors']]
    performers = [m.alloc(0x80) for _ in c['performers']]
    bodies = [m.alloc(0x20) for _ in c['performers']]
    owners = [m.alloc(0x100) for _ in c['performers']]
    slots = [m.alloc(0x200) for _ in c['slots']]
    layout, groups, parameters = m.alloc(0x40), m.alloc(0x40), m.alloc(0x40)
    m.put(org+0x18,layout);m.put(layout+0x28,groups);m.put(org+0x1c,parameters)
    for i, slot in enumerate(c['slots']):
        m.put(groups+4*i,i);m.put(parameters+4*i,slots[i] if slot else 0)
        if slot:
            m.put(slots[i]+0x174,slot['flags']);m.put(slots[i]+0x1e4,slot['value'])
    for i, actor in enumerate(c['actors']):
        m.put(actors[i]+0x74,performers[actor-1] if actor else 0)
    for i, performer in enumerate(c['performers']):
        m.put(performers[i]+4,owners[i]);m.put(performers[i]+0x60,performer['value'])
        if performer['health']:
            m.put(owners[i]+0x60,bodies[i]);m.put_words(bodies[i]+0x10,performer['health'])
    for i, array in enumerate(c['arrays']):
        m.put_words(arrays[i],[actors[a-1] if a else 0 for a in array])
    m.put(org+0x28,arrays[c['array']]);m.put(org+0x2c,c['count'])
    m.put(org+0x68,c['initial'][0]);m.put(org+0x6c,c['initial'][1])
    events, stores, counts = [], [], {}
    def change(a):
        kind=a['kind']
        if kind=='count':m.put(org+0x2c,a['value'])
        elif kind=='array':m.put(org+0x28,arrays[a['value']])
        elif kind=='member':m.put(arrays[a['array']]+4*a['index'], actors[a['value']-1] if a['value'] else 0)
        elif kind=='performer':m.put(actors[a['member']-1]+0x74,performers[a['value']-1] if a['value'] else 0)
        elif kind=='value':m.put(performers[a['performer']-1]+0x60,a['value'])
        elif kind=='current':m.put(org+0x68,a['value'])
        elif kind=='maximum':m.put(org+0x6c,a['value'])
        else:raise AssertionError(kind)
    def event(name,*args):
        events.append([name,*args]);counts[name]=counts.get(name,0)+1
        for a in c.get('mutations',[]):
            if a['event']==name and a['occurrence']==counts[name]:change(a)
        stop=c.get('stop')
        if stop and stop['event']==name and stop['occurrence']==counts[name]:raise Dependency(name)
    def code(uc,pc,size,_):
        r=uc.reg_read
        if pc in [0x5788b3,0x578926]:event('count')
        elif pc in [0x5788bb,0x5788eb]:event('store_current')
        elif pc in [0x5788c2,0x578923]:event('store_maximum')
        elif pc in [0x5788cb,0x5788e8]:event('array')
        elif pc in [0x5788d0,0x5788ee]:event('member',arrays.index(r(UC_X86_REG_ECX)),r(UC_X86_REG_EDI))
        elif pc in [0x5788d7,0x5788f1] and r(UC_X86_REG_EAX):event('performer',actors.index(r(UC_X86_REG_EAX))+1)
        elif pc==0x57dc40:event('health',performers.index(r(UC_X86_REG_ECX))+1)
        elif pc in [0x57dc50,0x57dc5a]:event('value',performers.index(r(UC_X86_REG_ECX))+1)
        elif pc==0x5788f4 and r(UC_X86_REG_EDX):event('value',performers.index(r(UC_X86_REG_EDX))+1)
        elif pc==0x5788e5:event('current')
        elif pc==0x578920:event('maximum')
        elif pc==0x5788f9:event('slot',r(UC_X86_REG_EDI))
        elif pc==0x57890c:event('flags',slots.index(r(UC_X86_REG_EAX)))
        elif pc==0x57891a:event('slot_value',slots.index(r(UC_X86_REG_EAX)))
    def writes(uc,access,address,size,value,_):
        if address in [org+0x68,org+0x6c]:stores.append([address-org,value&0xffffffff])
    m.uc.hook_add(UC_HOOK_CODE,code)
    m.uc.hook_add(UC_HOOK_MEM_WRITE,writes)
    outcome='ok'
    try:m.call(0x5788b0,[],org)
    except Dependency as error:outcome=str(error)
    except AssertionError:
        if not m.null_accesses:raise
        assert c['name'] in ['vanished_member_after_current','vanished_performer_after_current']
        expected=('0x5788f1',0x74,4) if c['name']=='vanished_member_after_current' else ('0x5788f4',0x60,4)
        assert m.null_accesses==[expected],m.null_accesses
        outcome='invalid'
    return {'outcome':outcome,'values':[m.read(org+0x68),m.read(org+0x6c)],'events':events,'stores':stores}

def cases():
    base={'name':'mixed','count':4,'array':0,'arrays':[[1,2,0,3],[2,1,0,3]],
          'actors':[1,2,0], 'performers':[{'value':bits(19.5),'health':[bits(1),bits(3)]},
              {'value':bits(5.25),'health':None}],
          'slots':[None,None,{'flags':32,'value':bits(7)},None],
          'initial':[bits(-123),bits(999)]}
    yield base
    for count in [-2,0,1,2,3]:yield dict(base,name='count_'+str(count),count=count)
    for value in [0,0x80000000,0x00800000,0x3f800001,0x4b800001,0x7e000001]:
        c=copy.deepcopy(base);c['name']='numeric_'+str(value)
        c['performers'][0]={'value':value,'health':[0x3f7fffff,0x3f800001]};yield c
    for flags in [0,1,31,64,0xffffffff]:
        c=copy.deepcopy(base);c['name']='flags_'+str(flags);c['slots'][2]['flags']=flags
        if flags&32==0:c['slots'][2]['value']=0x7fc00000
        yield c
    for name,initial,action in [
        ('initial_zero_stays_zero',0,{'event':'store_current','occurrence':1,'kind':'count','value':4}),
        ('initial_positive_still_enters',4,{'event':'store_current','occurrence':1,'kind':'count','value':0}),
        ('count_shrinks_after_current',4,{'event':'store_current','occurrence':2,'kind':'count','value':1}),
        ('count_grows_after_current',1,{'event':'store_current','occurrence':2,'kind':'count','value':4}),
        ('array_before_capture',4,{'event':'current','occurrence':1,'kind':'array','value':1}),
        ('array_after_capture',4,{'event':'store_current','occurrence':2,'kind':'array','value':1}),
        ('fresh_member_after_current',4,{'event':'store_current','occurrence':2,'kind':'member','array':0,'index':0,'value':2}),
        ('fresh_performer_after_current',4,{'event':'store_current','occurrence':2,'kind':'performer','member':1,'value':2}),
        ('vanished_member_after_current',4,{'event':'store_current','occurrence':2,'kind':'member','array':0,'index':0,'value':0}),
        ('vanished_performer_after_current',4,{'event':'store_current','occurrence':2,'kind':'performer','member':1,'value':0}),
        ('fresh_value_after_current',4,{'event':'store_current','occurrence':2,'kind':'value','performer':1,'value':bits(77)}),
        ('maximum_read_after_value',4,{'event':'value','occurrence':2,'kind':'maximum','value':bits(13)}),
    ]:yield dict(base,name=name,count=initial,mutations=[action])
    # No-Performer/vacancy branches retain the previously captured array pointer.
    c=copy.deepcopy(base);c['name']='vacancy_does_not_recapture';c['arrays'][0]=[0,1,0,3]
    c['slots'][0]={'flags':0,'value':0x7fc00000}
    c['mutations']=[{'event':'slot','occurrence':1,'kind':'array','value':1}];yield c
    c=copy.deepcopy(base);c['name']='no_performer_does_not_recapture';c['arrays'][0]=[3,1,0,3]
    c['mutations']=[{'event':'performer','occurrence':1,'kind':'array','value':1}];yield c
    # Explicit required-service cutpoints preserve every earlier native store.
    for event,count in [('count',1),('count',2),('store_current',1),('store_current',2),
        ('store_maximum',1),('store_maximum',2),('array',1),('array',2),('member',1),
        ('member',2),('performer',1),('performer',2),('health',1),('value',1),('value',2),
        ('current',1),('maximum',1),('slot',1),('flags',1),('slot_value',1)]:
        yield dict(base,name='stop_'+event+'_'+str(count),stop={'event':event,'occurrence':count})

def main():
    rows=[]
    for image in ['original','v14']:
        for pc,cw in MODES:
            for c in cases():rows.append({'image':image,'precision':pc,'input':c,'output':run(c,image,cw)})
    out={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.data),'v14_sha256':sha(ACCEPTED.data),
         'producer_sha256':sha(Path(__file__).read_bytes()),
         'harness_sha256':sha((ROOT/'research/phase8-economy/harness.py').read_bytes()),
         'ranges':pin({'organization':(0x5788b0,0x578931),'performer':(0x57dc40,0x57dc5e),
                       'one':(0x9f32e0,0x9f32e4)}),'cases':rows}
    path=HERE/'native-borrowed-combat-value.json'
    if '--write' in sys.argv:path.write_text(json.dumps(out,indent=2)+'\n')
    else:assert json.loads(path.read_text())==out,'native borrowed combat value differs'
    print(json.dumps({'cases':len(rows),'sha256':sha(path.read_bytes())}))
if __name__=='__main__':main()

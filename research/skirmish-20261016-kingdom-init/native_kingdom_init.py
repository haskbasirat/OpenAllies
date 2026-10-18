"""Execute full original/v14 4E5DF0 coordinator at all x87 precisions.

Required actor-init, legacy scheduling, restriction, recruit ownership, SAI,
world registration and relation callbacks are explicit recorded fixture seams.
Native branches, flow stores, Team links/count, category iterator and nullable
major iterator execute unchanged. Failure injection stops at a callback entry;
it proves prefix ordering, not C++ exception unwinding. No callback is described
as a completed runtime binding by this oracle.
"""
from pathlib import Path
import sys,json,copy
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as nm
from native_machine import Machine,sha,instructions
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL,V14=nm.original,nm.modified
KINGDOM=1000

def base():
    return {'name':'base','actor_success':True,'argument':0,'world_version':23,'word184':0,
      'resource_count':2,'flows':[[11,12,13],[21],[31,32]],'stock':[41,42,43],
      'team':{'members':[7,8],'count':2},'word190':0,'sai_active':False,
      'categories':[[1,2],[3]],'definitions':[
        {'id':1,'mask':256,'links':[True,False,False],'allowed':True},
        {'id':2,'mask':256,'links':[False,True,False],'allowed':False},
        {'id':3,'mask':256,'links':[False,False,True],'allowed':True},
        {'id':4,'mask':0,'links':[False,False,False],'allowed':True}],
      'majors':[1,None,2],'register_majors':[1,None,2,KINGDOM],
      'related':{'1':True,'2':False,str(KINGDOM):True},'mutations':[],'fail_at':None}

def cases():
    out=[]
    def add(name,**kw):
        c=base();c.update(kw);c['name']=name;out.append(c);return c
    add('base')
    add('actor_false',actor_success=False,argument=77)
    add('legacy_before_flows',world_version=21,argument=9)
    add('version22_no_legacy',world_version=22)
    add('legacy_word_present',world_version=1,word184=55)
    add('unsigned_version',world_version=0xffffffff)
    add('resources_negative',resource_count=-1,team=None)
    add('resources_empty',resource_count=0,flows=[[],[],[]],team={'members':[],'count':0})
    add('resource_grows_after_first_cell',mutations=[{'event':'flow:416:0','resource_count':4}])
    add('resource_shrinks_midrow',resource_count=4,mutations=[{'event':'flow:428:0','resource_count':0}])
    add('team_count_wrap',team={'members':[],'count':0xffffffff})
    add('active_sai',sai_active=True)
    add('minor_skips_active_sai',word190=2,sai_active=True)
    add('actor_init_mutates_gate',world_version=23,mutations=[{'event':'actor_init','world_version':21,'word184':0,'resource_count':1}])
    c=add('predicate_short_circuit');c['definitions'][0]['mask']=0;c['definitions'][1]['links']=[False,False,False]
    c=add('no_element_null_backreference');c['definitions'][0]['mask']=0;c['definitions'][0]['null_links']=True
    c=add('element_null_backreference');c['definitions'][0]['null_links']=True
    add('category_empty_first',categories=[[],[3]])
    add('category_empty_second',categories=[[1,2],[]])
    add('node_link_mutation',mutations=[{'event':'allowed:1','next_node':[1,4]},{'event':'allowed:1','next_node':[4,2]}])
    add('major_grows_in_refresh',mutations=[{'event':'refresh228:1','majors':[1,None,2,KINGDOM,3]}],related={'1':True,'2':False,'3':True,str(KINGDOM):True})
    add('major_shrinks_in_refresh',mutations=[{'event':'refresh228:1','majors':[1]}])
    add('major_nulls_after_callback',mutations=[{'event':'related:1','majors':[1,None,None,KINGDOM]}])
    for point in ['actor_init','diagnostic','legacy','allowed:1','recruit:1','sai','register','related:1','refresh228:1','refresh22c:1']:
        add('failure_'+point.replace(':','_'),fail_at=point,world_version=21,sai_active=True)
    return out

def run(exe,pc,c):
    nm.original=exe;m=Machine(pc);events=[];recruits=[];error=None
    kingdom=m.alloc(0x400);world=m.alloc(0x300);data=m.alloc(0x100);sai=m.alloc(0x40)
    m.w(0xb36534,world);m.w(0xb36530,data);m.w(0xb36528,sai)
    m.w(world+0x2c,c['world_version']);m.w(data+0x38,c['resource_count']);m.w(kingdom+0x184,c['word184']);m.w(kingdom+0x190,c['word190']);m.wb(sai+0x20,int(c['sai_active']))
    ids={KINGDOM:kingdom};reverse={kingdom:KINGDOM}
    for value in [1,2,3,7,8]:
        p=m.alloc(0x400);ids[value]=p;reverse[p]=value
    major_array=m.alloc(32*4);m.w(world+0x154,major_array);m.w(world+0x15c,32)
    def set_majors(values):
        m.w(world+0x158,len(values))
        for i,value in enumerate(values):m.w(major_array+i*4,ids[value] if value is not None else 0)
    set_majors(c['majors'])
    flow_headers={};flow_bases={}
    for offset,values in zip([0x1a0,0x1ac,0x1b8],c['flows']):
        p=m.alloc(max(1,len(values))*4);m.w(kingdom+offset,p);m.w(kingdom+offset+4,len(values));m.w(kingdom+offset+8,len(values));flow_headers[kingdom+offset]=offset;flow_bases[p]=(offset,max(1,len(values)))
        for i,value in enumerate(values):m.w(p+i*4,value)
    stock=m.array(c['stock']);m.w(kingdom+0x1c4,stock);m.w(kingdom+0x1c8,len(c['stock']))
    team=None
    if c['team'] is not None:
        team=m.alloc(0x80);m.w(kingdom+0x1ec,team);head=0;tail=0
        for value in reversed(c['team']['members']):
            node=m.array([ids[value],head]);head=node
            if not tail:tail=node
        m.w(team+0x44,head);m.w(team+0x48,tail);m.w(team+0x68,c['team']['count'])
    manager=m.alloc(0x100);m.w(0xb39f58,manager);category_heads=m.alloc(43*4);m.w(manager+0x5c,category_heads)
    defs={};nodes={};def_reverse={}
    for row in c['definitions']:
        p=m.alloc(0x300);defs[row['id']]=p;def_reverse[p]=row['id'];m.w(p+0x174,row['mask']);links=m.alloc(16);m.w(p+0x2c8,0 if row.get('null_links') else links);m.w(links,int(row['links'][0]));m.w(links+12,int(row['links'][1]));m.w(p+0x17c,int(row['links'][2]));nodes[row['id']]=m.array([p,0,0])
    for cat,values in zip([19,42],c['categories']):
        m.w(category_heads+4*cat,nodes[values[0]] if values else 0)
        for a,b in zip(values,values[1:]):m.w(nodes[a]+8,nodes[b])
    def mutate(key):
        for change in c['mutations']:
            if change['event']!=key:continue
            for field,offset,owner in [('resource_count',0x38,data),('world_version',0x2c,world),('word184',0x184,kingdom),('word190',0x190,kingdom)]:
                if field in change:m.w(owner+offset,change[field])
            if 'sai_active'in change:m.wb(sai+0x20,int(change['sai_active']))
            if 'majors'in change:set_majors(change['majors'])
            if 'next_node'in change:
                a,b=change['next_node'];m.w(nodes[a]+8,nodes[b] if b is not None else 0)
    def event(key,**args):
        nonlocal error
        events.append({'event':key,**args})
        if c['fail_at']==key:error=key;m.u.emu_stop();return False
        mutate(key);return True
    def actor(mm):
        assert mm.ecx()==kingdom
        if event('actor_init',argument=mm.arg(1)):mm.ret(int(c['actor_success']),pop=4)
    m.seam(0x4d7230,actor)
    string=m.alloc(20)+16;m.w(string-4,2)
    def name(mm):out=mm.arg(1);mm.w(out,string);mm.ret(out,pop=4)
    m.seam(0x4bedf0,name)
    m.seam(0x404f60,lambda mm:mm.ret() if event('diagnostic') else None)
    def legacy(mm):
        assert mm.ecx()==kingdom+0x180 and mm.arg(1)==0
        if event('legacy'):mm.w(kingdom+0x184,0x777);mm.ret(pop=4)
    m.seam(0x50ac00,legacy)
    m.seam(0x6f7810,lambda mm:mm.ret(mm.alloc(mm.arg(1))))
    m.seam(0x6f7430,lambda mm:mm.ret())
    def grow(mm):
        header=mm.ecx();capacity=mm.arg(1);old=mm.r(header);length=mm.r(header+4);p=mm.alloc(capacity*4);mm.u.mem_write(p,b'\xa5'*(capacity*4));mm.u.mem_write(p,bytes(mm.u.mem_read(old,length*4)));mm.w(header,p);mm.w(header+8,capacity)
        if header in flow_headers:flow_bases[p]=(flow_headers[header],capacity)
        mm.ret(pop=4)
    m.seam(0x4849f0,grow)
    def writes(u,access,address,size,value,_):
        eip=u.reg_read(UC_X86_REG_EIP)
        if eip in [0x4e5f14,0x4e5f51,0x4e5f8c]:
            offset={0x4e5f14:0x1a0,0x4e5f51:0x1ac,0x4e5f8c:0x1b8}[eip];index=(address-m.r(kingdom+offset))//4;assert size==4 and value==0;event(f'flow:{offset}:{index}')
        elif eip==0x4e5fda:event('team_prepend')
        elif eip==0x4e5fea:event('team_count',value=value)
    m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
    allowed={r['id']:r['allowed']for r in c['definitions']}
    def restriction(mm):
        assert mm.ecx()==kingdom;definition=def_reverse[mm.arg(1)]
        if event(f'allowed:{definition}'):mm.ret(int(allowed[definition]),pop=4)
    m.seam(0x4e1810,restriction)
    def recruit(mm):
        assert mm.ecx()==kingdom+0x1fc;item=mm.arg(1);definition=def_reverse[mm.r(item+4)];available=mm.rb(item+8);assert mm.r(item)==0xa29650
        if event(f'recruit:{definition}',available=available):recruits.append({'definition':definition,'available':available});mm.ret(pop=4)
    m.seam(0x4e4c30,recruit)
    def register_sai(mm):
        assert mm.arg(1)==kingdom
        if event('sai'):mm.ret(pop=4)
    m.seam(0x79e170,register_sai)
    def register(mm):
        assert mm.ecx()==world and mm.arg(1)==kingdom
        if event('register'):set_majors(c['register_majors']);mm.ret(pop=4)
    m.seam(0x4f8af0,register)
    def related(mm):
        other=reverse[mm.ecx()];assert mm.arg(1)==kingdom
        if event(f'related:{other}'):mm.ret(int(c['related'].get(str(other),False)),pop=4)
    virtual=m.stub(related)
    for p in ids.values():
        vt=m.alloc(0x13c);m.w(vt+0x138,virtual);m.w(p,vt)
    for address,label,offset in [(0x54eba0,'refresh228',0x228),(0x559600,'refresh22c',0x22c)]:
        def refresh(mm,label=label,offset=offset):
            other=reverse[mm.arg(1)];assert mm.ecx()==ids[other]+offset
            if event(f'{label}:{other}'):mm.ret(pop=4)
        m.seam(address,refresh)
    try:
        returned=m.call(0x4e5df0,kingdom,[c['argument']])&0xff;status={'returned':bool(returned),'error':None}
    except AssertionError:
        if error:status={'returned':None,'error':'callback:'+error}
        elif m.null_accesses:
            fault_pc=m.null_accesses[0][0]
            assert fault_pc in [0x4e603f,0x46c8c3],m.null_accesses
            status={'returned':None,'error':'null_recruit_backreference' if fault_pc==0x4e603f else 'null_definition_cursor'}
        else:raise
    team_state=None
    if team:
        values=[];head=m.r(team+0x44);node=head;tail=0
        while node:values.append(reverse[m.r(node)]);tail=node;node=m.r(node+4)
        assert m.r(team+0x48)==tail
        team_state={'members':values,'count':m.r(team+0x68)}
    snapshot={'events':events,'status':status,'flows':[[m.r(m.r(kingdom+off)+i*4)for i in range(m.r(kingdom+off+4))]for off in [0x1a0,0x1ac,0x1b8]],'stock':[m.r(stock+i*4)for i in range(len(c['stock']))],'team':team_state,'recruits':recruits,'word184':m.r(kingdom+0x184),'resource_count':((m.r(data+0x38)^0x80000000)-0x80000000),'majors':[reverse.get(m.r(major_array+i*4))for i in range(m.r(world+0x158))]}
    for a,b in m.visited.items():assert b==ORIGINAL.range(a,len(b))==V14.range(a,len(b)),hex(a)
    return snapshot,m.visited

def main():
    rows=[];visited={};executions=0
    for c in cases():
        for pc in nm.PRECISIONS:
            expected=None
            for exe in [ORIGINAL,V14]:
                actual,seen=run(exe,pc,c);visited.update(seen);executions+=1
                if expected is None:expected=actual
                else:assert expected==actual,(c['name'],pc)
            rows.append({'case':c,'precision':pc,'expected':expected})
        print(c['name'],expected['status'],flush=True)
    nm.original=ORIGINAL
    doc={'schema':1,'scope':__doc__,'original_sha256':sha(ORIGINAL.b),'v14_sha256':sha(V14.b),'producer_sha256':sha(Path(__file__).read_bytes()),'executions':executions,'cases':rows,'instructions':instructions(visited)}
    path=HERE/'native-kingdom-init.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(len(rows),'cases',executions,'executions',sha(path.read_bytes()))
if __name__=='__main__':main()

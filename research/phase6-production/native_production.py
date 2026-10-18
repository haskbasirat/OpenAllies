"""Run original structure queue routines, arithmetic and resource vectors.

Actor/research creation, scheduler linkage, availability predicates, presentation
strings/events and allocators are required boundary callbacks. Queue ordering,
pause/deadline arithmetic, resource filtering/equality/copy, duration arithmetic,
stock debits/refunds and coupled upkeep arithmetic execute original instructions.
"""
from pathlib import Path
import copy, hashlib, importlib.util, json, random, struct, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
spec = importlib.util.spec_from_file_location('economy_oracle', ROOT/'research/evidence/economy-phase2-oracle.py')
economy = importlib.util.module_from_spec(spec); spec.loader.exec_module(economy)
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

bits=economy.bits
sha=economy.sha

class ProductionMachine(economy.Machine):
    COMPONENT=0x2060000
    ACTOR=0x2061000
    VTABLE=0x2062000
    WORLD=0x2063000
    QUEUE=0x2064000
    DEFINITIONS=0x2068000
    ITEMS=0x2070000
    RATE=0x2065000
    def __init__(self, case, cw):
        self.case=copy.deepcopy(case)
        self.events=[]; self.executed={}; self.allocations=[]; self.quote_index=0
        super().__init__(case['catalog'],case['economy'],case['limits'],cw)
        self.put(0x2022000+0x24,0x2021000)
        self.put(0xb36534,self.WORLD)
        self.put(0x2050000+0x1c0,0x2051008)
        self.uc.mem_write(0x3000200,b'\xd9\x05'+struct.pack('<I',self.RATE)+b'\xc2\x04\x00')
        self.put(self.ACTOR,self.VTABLE)
        self.put(self.VTABLE+0x108,0x3000100)
        self.put(self.VTABLE+0x24,0x3000110)
        self.put(self.ACTOR+4,self.DEFINITIONS)
        self.put(self.ACTOR+0x14,37)
        self.put(self.ACTOR+0x94,self.COMPONENT)
        self.put(self.ACTOR+0xa0,self.ACTOR+0x300)
        self.put(self.ACTOR+0xb4,self.ACTOR+0x400)
        self.put(self.ACTOR+0x304,self.ACTOR)
        self.put(self.ACTOR+0x404,self.ACTOR)
        self.put(self.COMPONENT+4,self.ACTOR)
        self.put(self.COMPONENT+0x14,self.QUEUE)
        self.put(self.COMPONENT+0x1c,128)
        for offset,key in [(0x20,'deadline_bits'),(0x24,'paused_at_bits'),(0x28,'pause_reason'),(0x2c,'duration_bits')]:
            self.put(self.COMPONENT+offset,case['queue'][key])
        self.put(self.COMPONENT+0xc,int(case['queue']['scheduled']))
        self.put(self.COMPONENT+0x18,len(case['queue']['items']))
        self.put(self.WORLD+0x198,case['world']['next_item_id'])
        self.put(0x2067004,100)
        self.put(0x2067400+0xd8,0x2067600)
        self.item_pointers={}
        for index,item in enumerate(case['queue']['items']):
            pointer=self.ITEMS+index*0x100
            self.write_item(pointer,item)
            self.put(self.QUEUE+index*4,pointer)
        self.update(case['world'])

    def allocate(self,size):
        address=self.heap; self.heap+=(max(size,1)+15)&~15
        assert self.heap<0x20f0000
        self.uc.mem_write(address,bytes(max(size,1)))
        self.allocations.append(address)
        return address

    def definition(self,id):
        return self.DEFINITIONS+id*0x400 if id else 0

    def resource(self,address,values):
        data=self.allocate(4*len(values))
        self.put(address,0xa26f44);self.put(address+4,data)
        self.put(address+8,len(values));self.put(address+12,len(values))
        self.vector(data,values)

    def write_item(self,p,item):
        self.item_pointers[item['id']]=p
        for offset,key in [(0,'id'),(4,'kind'),(12,'organization_layout'),(0x4c,'organization_slot')]: self.put(p+offset,item[key])
        self.put(p+8,self.definition(item['actor_definition']))
        self.put(p+0x18,self.definition(item['property']))
        self.vector(p+0x10,item['location_bits'])
        for offset,key in [(0x1c,'paid_cost'),(0x2c,'requested_upkeep'),(0x3c,'current_upkeep')]:self.resource(p+offset,item[key])

    def item(self,p):
        return {'id':self.read(p),'kind':self.read(p+4),
                'actor_definition':(self.read(p+8)-self.DEFINITIONS)//0x400 if self.read(p+8) else 0,
                'organization_layout':self.read(p+0xc),'location_bits':[self.read(p+0x10),self.read(p+0x14)],
                'property':(self.read(p+0x18)-self.DEFINITIONS)//0x400 if self.read(p+0x18) else 0,
                'paid_cost':self.values(self.read(p+0x20)),
                'requested_upkeep':self.values(self.read(p+0x30)),
                'current_upkeep':self.values(self.read(p+0x40)),
                'organization_slot':self.read(p+0x4c)}

    def queue(self):
        return {'items':[self.item(self.read(self.read(self.COMPONENT+0x14)+i*4)) for i in range(self.read(self.COMPONENT+0x18))],
                'deadline_bits':self.read(self.COMPONENT+0x20),'duration_bits':self.read(self.COMPONENT+0x2c),
                'paused_at_bits':self.read(self.COMPONENT+0x24),'pause_reason':struct.unpack('<i',struct.pack('<I',self.read(self.COMPONENT+0x28)))[0],
                'scheduled':self.read(self.COMPONENT+0xc)!=0}

    def update(self,values):
        self.case['world'].update(values)
        w=self.case['world']
        self.put(self.WORLD+0xe0,w['now_bits']);self.put(self.WORLD+0xe8,w['delta_bits'])
        self.put(self.ACTOR+0x104,0x80 if w['disabled'] else 0)
        self.put(0x2051008,int(w['instant_production']))
        for id,d in w['definitions'].items():
            p=self.definition(int(id));self.put(p+0x174,0x80 if d['organization'] else 0)
            self.put(p+0x318,d['recruit_time_bits']);self.put(p+0x28,d['research_duration_bits'])
            self.put(p+0x2f0,0x2067000 if d.get('default_layout',0) else 0)

    def ret(self,value=0,pop=0):
        stack=self.uc.reg_read(UC_X86_REG_ESP)
        self.uc.reg_write(UC_X86_REG_EAX,value&0xffffffff)
        self.uc.reg_write(UC_X86_REG_EIP,self.read(stack))
        self.uc.reg_write(UC_X86_REG_ESP,stack+4+pop)

    def event(self,name,**fields):
        self.events.append({'name':name,**fields})

    def string(self,this):
        p=self.allocate(48);self.put(p+12,100);self.put(this,p+16)
        return this

    def observe(self,uc,address,size,_):
        if economy.ORIGINAL.base<=address<economy.ORIGINAL.base+economy.ORIGINAL.size:
            raw=economy.ORIGINAL.at(address,size)
            assert raw==economy.ACCEPTED.at(address,size),hex(address)
            self.executed[address]=raw
        stack=uc.reg_read(UC_X86_REG_ESP);this=uc.reg_read(UC_X86_REG_ECX)
        arg=lambda i:self.read(stack+4+4*i)
        w=self.case['world']
        if address==0x6f7810:self.ret(self.allocate(arg(0)))
        elif address==0x6f7430:self.ret()
        elif address==0x3000100:self.ret(self.KINGDOM if w['kingdom'] else 0)
        elif address==0x3000110:self.ret(self.string(arg(0)),4)
        elif address==0x401b00:self.ret(self.string(this),4)
        elif address==0x4b9f40:self.ret(self.string(self.allocate(4)),8)
        elif address==0x401560:self.put(this,self.read(arg(0)));self.ret(this,4)
        elif address==0x40c230:
            if self.read(stack)==0x582a41:self.event('notify_unaffordable')
            else:self.event('notify_pause',reason=2 if self.read(self.ACTOR+0x104)&0x80 else 1)
            self.ret()
        elif address==0x5c4d10:self.ret(self.string(arg(0)))
        elif address==0x77dc50:self.ret(pop=8)
        elif address==0x5827b0:
            self.event('recruitability',definition=(arg(0)-self.DEFINITIONS)//0x400,layout=7 if arg(1) else 0)
            self.ret(w['recruitability'],16)
        elif address==0x4cf110:self.ret(0)
        elif address==0x56f850:
            values=w['cost_quotes'][min(self.quote_index,len(w['cost_quotes'])-1)];self.quote_index+=1
            self.event('quote_cost',values=values)
            self.vector(self.read(arg(0)+4),values);self.ret(arg(0))
        elif address==0x56f580:
            self.event('quote_upkeep',values=w['quoted_upkeep'])
            self.vector(self.read(arg(0)+4),w['quoted_upkeep']);self.ret(arg(0))
        elif address==0x4e1690:self.event('debit',values=self.values(self.read(arg(0))))
        elif address==0x4e2e00:
            self.event('reserve_recruit',definition=(arg(0)-self.DEFINITIONS)//0x400,layout=0);self.ret(1,12)
        elif address==0x4e4050:self.event('reserve_recruit',definition=1,layout=7);self.ret(1,8)
        elif address in (0x404ea0,0x404ed0):self.ret()
        elif address==0x4d64f0:self.event('actor_state',state=arg(0),enabled=bool(arg(1)));self.ret(pop=8)
        elif address==0x50b250:self.event('cancel_schedule');self.put(self.COMPONENT+0xc,0);self.ret()
        elif address==0x50ac00:self.event('schedule_zero',delay=arg(0));self.put(self.COMPONENT+0xc,1);self.ret(pop=4)
        elif address==0x45d5b0:
            id=(this-self.DEFINITIONS)//0x400
            rate=w['definitions'][str(id)]['recruit_rate_bits'];self.event('recruit_rate',definition=id,value=rate)
            self.put(self.RATE,rate);uc.reg_write(UC_X86_REG_EIP,0x3000200)
        elif address==0x582630:self.event('can_reserve_upkeep',notify=bool(arg(3)&255));self.ret(int(w['can_reserve_upkeep']))
        elif address==0x4e48c0:
            self.event('adjust_upkeep',values=self.values(self.read(arg(0))),add=bool(arg(1)))
        elif address==0x4ce830:self.event('requires_supply');self.ret(int(w['requires_supply']))
        elif address==0x4d0c90:self.event('in_supply');self.ret(int(w['in_supply']))
        elif address==0x4cffd0:self.ret(0x2067400 if w['organization_manager'] else 0)
        elif address==0x574bf0:
            self.event('can_enroll_organization',definition=(arg(0)-self.DEFINITIONS)//0x400)
            self.ret(int(w['organization_allowed']),8)
        elif address==0x574db0:self.event('register_organization_item',id=arg(0));self.ret(pop=12)
        elif address==0x573d10:self.event('detach_organization_slot',id=arg(0));self.ret(w['detached_slot'],4)
        elif address in (0x582e80,0x582190,0x584120):
            kind={0x582e80:0,0x582190:2,0x584120:1}[address]
            fields={'kind':kind,'success':w['fulfill_success']}
            if kind in (0,2):fields['organization_slot']=arg(5 if kind==0 else 7)
            self.event('fulfill',**fields);self.ret(int(w['fulfill_success']),{0:24,1:8,2:32}[kind])
        elif address in (0x581d40,0x581d90,0x583f30):
            self.event('refund',kind={0x581d40:0,0x581d90:2,0x583f30:1}[address])
        elif address in (0x4e2e60,0x4e40b0,0x4e2360):self.ret(pop={0x4e2e60:8,0x4e40b0:4,0x4e2360:4}[address])
        elif address==0x589200:self.event('release_item',id=self.read(this),organization_slot=self.read(this+0x4c))
        elif address==0x4e1f60:
            pass # original complete shortage arithmetic executes; trace not duplicated

    def action(self,action):
        self.events=[]
        if 'world' in action:self.update(action['world'])
        address={'start':0x589420,'process':0x589f90,'cancel':0x589bd0,'request':0x5836e0}[action['kind']]
        args=[action['id']] if action['kind']=='cancel' else []
        this=self.COMPONENT
        if action['kind']=='request':
            self.quote_index=0
            r=action['request'];self.put(0x2066000,0x2067000 if r['organization_layout'] else 0)
            if r['location_bits'] is not None:self.vector(0x2066100,r['location_bits'])
            args=[self.definition(r['actor_definition']),0x2066000,0x2066100 if r['location_bits'] is not None else 0]
            this=self.ACTOR+0x300
        self.uc.context_restore(self.context)
        self.put(self.STACK,self.STOP)
        for i,v in enumerate(args):self.put(self.STACK+4+i*4,v)
        self.uc.reg_write(UC_X86_REG_ESP,self.STACK);self.uc.reg_write(UC_X86_REG_ECX,this)
        preserved={register:0x77330000+index for index,register in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
        for register,value in preserved.items():self.uc.reg_write(register,value)
        try:
            self.uc.emu_start(address,self.STOP,count=500000)
        except Exception:
            print(self.case['name'],action,'pc',hex(self.uc.reg_read(UC_X86_REG_EIP)),
                  'this',hex(self.uc.reg_read(UC_X86_REG_ECX)),'stack',hex(self.uc.reg_read(UC_X86_REG_ESP)),self.events)
            raise
        assert self.uc.reg_read(UC_X86_REG_EIP)==self.STOP
        assert self.uc.reg_read(UC_X86_REG_ESP)==self.STACK+4+4*len(args)
        for register,value in preserved.items():assert self.uc.reg_read(register)==value
        result={'queue':self.queue(),'economy':self.state(),'events':self.events}
        if action['kind']=='cancel':result['index']=struct.unpack('<i',struct.pack('<I',self.uc.reg_read(UC_X86_REG_EAX)))[0]
        if action['kind']=='request':result['accepted']=bool(self.uc.reg_read(UC_X86_REG_EAX)&255)
        result['next_item_id']=self.read(self.WORLD+0x198)
        return result

def item(id,kind=1):
    return {'id':id,'kind':kind,'actor_definition':1 if kind in (0,2) else 0,'organization_layout':0,
            'location_bits':[bits(3),bits(4)],'property':2 if kind==1 else 0,
            'paid_cost':[bits(10),0,0],'requested_upkeep':[0,bits(1),bits(2)],'current_upkeep':[0,0,0],'organization_slot':0}

def case(name,items,actions,**world):
    state={'items':items,'deadline_bits':bits(20),'duration_bits':bits(5),'paused_at_bits':bits(-1),'pause_reason':-1,'scheduled':False}
    w={'now_bits':bits(10),'delta_bits':bits(.1),'instant_production':False,'disabled':False,
       'requires_supply':True,'in_supply':True,'can_reserve_upkeep':True,'fulfill_success':True,'kingdom':True,
       'next_item_id':100,'recruitability':0,'cost_quotes':[[bits(10),0,0]],'quoted_upkeep':[0,bits(1),bits(2)],
       'organization_manager':False,'organization_allowed':True,'detached_slot':23,
       'definitions':{'1':{'organization':False,'recruit_time_bits':bits(4),'research_duration_bits':0,'recruit_rate_bits':bits(.25)},
                      '2':{'organization':False,'recruit_time_bits':0,'research_duration_bits':bits(5),'recruit_rate_bits':0}}}
    w.update(world)
    c=economy.catalog(['Stockpiled','Upkeep','LimitedConsumed'],[[0,0,0],[2,0,0],[0,0,0]])
    e={'raw_provided':[0,bits(5),bits(100)],'effective_provided':[0,bits(5),bits(100)],'consumed':[0,0,0],'stockpiled':[bits(50),0,0]}
    return {'name':name,'queue':state,'world':w,'catalog':c,'economy':e,
            'limits':{'provided_maximum':100,'stockpiled_maximum':bits(10000)},'actions':actions}

def main():
    scenarios=[
        case('research_complete',[item(11)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(15)}}]),
        case('recruit_complete',[item(11,0)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(14)}}]),
        case('recruit_cancel',[item(11,0)],[{'kind':'start'},{'kind':'cancel','id':11}]),
        case('research_cancel',[item(11)],[{'kind':'start'},{'kind':'cancel','id':11}]),
        case('research_pause',[item(11)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(12)}},{'kind':'process','world':{'now_bits':bits(13),'in_supply':True}},{'kind':'process','world':{'now_bits':bits(17)}}],in_supply=False),
        case('disabled_pause',[item(11)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(11)}},{'kind':'process','world':{'now_bits':bits(12),'disabled':False}}],disabled=True),
        case('limited_pause',[item(11,0)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(11)}},{'kind':'process','world':{'now_bits':bits(12),'can_reserve_upkeep':True}}],can_reserve_upkeep=False),
        case('failed_fulfillment',[item(11,0)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(14)}}],fulfill_success=False),
        case('fifo_completion',[item(11),item(12,0)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(15)}},{'kind':'process','world':{'now_bits':bits(19)}}]),
        case('cancel_nonhead',[item(11),item(12,0),item(13,2)],[{'kind':'start'},{'kind':'cancel','id':12},{'kind':'cancel','id':99}]),
    ]
    request={'kind':'request','request':{'actor_definition':1,'organization_layout':0,'location_bits':None}}
    scenarios += [
        case('request_complete',[],[request,{'kind':'process','world':{'now_bits':bits(14)}}]),
        case('request_cancel',[],[request,{'kind':'cancel','id':101}]),
        case('request_fifo',[],[request,request,{'kind':'process','world':{'now_bits':bits(14)}},{'kind':'cancel','id':102}]),
        case('request_no_kingdom',[],[request],kingdom=False),
        case('request_not_recruitable',[],[request],recruitability=3),
        case('request_unaffordable',[],[request],cost_quotes=[[bits(100),0,0]]),
        case('request_quote_changes',[],[request,{'kind':'cancel','id':101}],cost_quotes=[[bits(10),0,0],[bits(20),0,0]]),
        case('request_quote_exceeds_stock',[],[request],cost_quotes=[[bits(10),0,0],[bits(60),0,0]]),
    ]
    location=copy.deepcopy(request);location['request']['location_bits']=[bits(-4.75),bits(9.25)]
    scenarios += [
        case('request_location_complete',[],[location,{'kind':'process','world':{'now_bits':bits(14)}}]),
        case('request_location_cancel',[],[location,{'kind':'cancel','id':101}]),
        case('request_location_failed',[],[location,{'kind':'process','world':{'now_bits':bits(14)}}],fulfill_success=False),
        case('empty_start',[],[{'kind':'start'},{'kind':'process'},{'kind':'cancel','id':7}]),
        case('complete_before_disabled_gate',[item(11)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(15),'disabled':True}}]),
        case('supply_not_required',[item(11)],[{'kind':'start'}],requires_supply=False,in_supply=False),
    ]
    for label,enabled,allowed in [('organization_without_manager',False,True),('organization_manager',True,True),('organization_rejected',True,False)]:
        c=case(label,[],[request,{'kind':'process','world':{'now_bits':bits(14)}}],organization_manager=enabled,organization_allowed=allowed)
        c['world']['definitions']['1']['organization']=True
        c['world']['definitions']['1']['default_layout']=7
        scenarios.append(c)
    for rate in [0,-1,.3,3.14159,0.001]:
        c=case(f'rate_{rate}',[],[request,{'kind':'cancel','id':101}])
        c['world']['definitions']['1']['recruit_rate_bits']=bits(rate)
        scenarios.append(c)
    for now,delta in [(0,.2),(123.456,.0333333),(65536,.1),(1e-8,1e-5),(1e20,.1)]:
        c=case(f'instant_{now}_{delta}',[],[request,{'kind':'cancel','id':101}],now_bits=bits(now),delta_bits=bits(delta),instant_production=True)
        scenarios.append(c)
    scenarios += [
        case('research_failed_refund',[item(11)],[{'kind':'start'},{'kind':'process','world':{'now_bits':bits(15)}}],fulfill_success=False),
        case('cancel_head_preserves_pending_timer',[item(11),item(12)],[{'kind':'start'},{'kind':'cancel','id':11}]),
        case('unknown_kind_default_duration',[item(11,3)],[{'kind':'start'},{'kind':'cancel','id':11}]),
        case('item_id_wrap',[],[request,{'kind':'cancel','id':0}],next_item_id=0xffffffff),
    ]
    zero=item(11,0);zero['current_upkeep']=[0x80000000,0,0];zero['requested_upkeep']=[0,0,0]
    scenarios.append(case('signed_zero_upkeep_equality',[zero],[{'kind':'start'}]))
    for negative in [0,-1]:
        c=case(f'research_nonpositive_duration_{negative}',[item(11)],[{'kind':'start'}])
        c['world']['definitions']['2']['research_duration_bits']=bits(negative);scenarios.append(c)
    missing=item(11);missing['property']=0
    scenarios.append(case('due_research_missing_property',[missing],[{'kind':'process','world':{'now_bits':bits(20)}}]))
    rng=random.Random(0x589F90)
    for i in range(32):
        now=rng.uniform(.001,100000);delay=rng.uniform(.001,3);t1=now+delay;t2=t1+delay
        c=case(f'fractional_paused_duration_{i}',[item(11)],
               [{'kind':'start'},{'kind':'process','world':{'now_bits':bits(t1)}},{'kind':'process','world':{'now_bits':bits(t2),'disabled':False}}],
               now_bits=bits(now),disabled=True)
        c['world']['definitions']['2']['research_duration_bits']=bits(rng.uniform(10,100))
        scenarios.append(c)
    rows=[];executed={}
    for pc,cw in economy.MODES:
        for scenario in scenarios:
            machine=ProductionMachine(scenario,cw)
            steps=[]
            for action in scenario['actions']:
                steps.append({'input':action,'output':machine.action(action)})
            rows.append({'precision':pc,'input':scenario,'steps':steps});executed.update(machine.executed)
    output={'schema':1,'original_sha256':economy.ORIGINAL_SHA,'v14_sha256':economy.ACCEPTED_SHA,
            'producer_sha256':sha(Path(__file__).read_bytes()),'method':__doc__,'cases':rows,
            'abi_checked':'Every top-level step returns at the expected stack position and preserves EBX/ESI/EDI/EBP sentinels.',
            'visited_window_scope':'Includes intercepted boundary-function entries; their bodies are supplied as explicit callbacks rather than claimed executed.',
            'visited_windows': [{'address':hex(a),'bytes':b.hex()} for a,b in sorted(executed.items())]}
    (HERE/'native-production-fixtures.json').write_text(json.dumps(output,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({'cases':len(rows),'steps':sum(len(r['steps']) for r in rows),'sha256':sha((HERE/'native-production-fixtures.json').read_bytes())}))

if __name__=='__main__':main()

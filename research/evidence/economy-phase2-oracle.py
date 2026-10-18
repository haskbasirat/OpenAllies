"""Execute original resource functions, with real coupled arrays, in isolated x86.

No game launch/process writes. Arithmetic/branch decisions are never hooked.
Flow runs only through 4E9DA6; its two temporary allocations are the sole mocked
native calls. Shortage snapshots are read-only code-hook observations.
"""
from pathlib import Path
import hashlib, json, random, struct, sys

ROOT = Path(__file__).resolve().parents[2]
GAME = ROOT.parent
sys.path.insert(0, str(GAME / 'reconstruction/tools/python-packages'))
from unicorn import Uc, UC_ARCH_X86, UC_MODE_32, UC_HOOK_CODE
from unicorn.x86_const import *
from capstone import Cs, CS_ARCH_X86, CS_MODE_32

ORIGINAL_SHA = '5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26'
ACCEPTED_SHA = '3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
RANGES = {'shortage': (0x4e1f60, 0x4e2016), 'provider': (0x4e3940, 0x4e39cf),
          'consumption_vector': (0x4e48c0, 0x4e4924), 'provider_vector': (0x4e4930, 0x4e497f),
          'credit_stock': (0x4e39d0, 0x4e3a4c), 'debit_stock': (0x4e1690, 0x4e16c5),
          'can_afford': (0x4e20a0, 0x4e20f7), 'flow_prefix': (0x4e9c40, 0x4e9da6),
          'zero': (0x9f32dc, 0x9f32e0), 'seconds_to_minutes': (0xa1b268, 0xa1b26c)}
MODES = [('Single24', 0x007f), ('Double53', 0x027f), ('Extended64', 0x037f)]


def sha(data): return hashlib.sha256(data).hexdigest()
def bits(value): return struct.unpack('<I', struct.pack('<f', value))[0]
def f32(value): return struct.unpack('<f', struct.pack('<I', value))[0]


class Pe:
    def __init__(self, data):
        self.data = data
        p = struct.unpack_from('<I', data, 0x3c)[0]
        assert data[p:p+4] == b'PE\0\0'
        count = struct.unpack_from('<H', data, p+6)[0]
        optional_size = struct.unpack_from('<H', data, p+20)[0]
        self.base = struct.unpack_from('<I', data, p+24+28)[0]
        self.size = struct.unpack_from('<I', data, p+24+56)[0]
        self.header_size = struct.unpack_from('<I', data, p+24+60)[0]
        self.sections = []
        for i in range(count):
            s = p+24+optional_size+i*40
            virtual_size, rva, size, offset = struct.unpack_from('<4I', data, s+8)
            self.sections.append((rva, size, offset, virtual_size))

    def at(self, address, size):
        rva = address-self.base
        for va, length, offset, _ in self.sections:
            if va <= rva and rva+size <= va+length:
                return self.data[offset+rva-va:offset+rva-va+size]
        if rva+size <= self.header_size:
            return self.data[rva:rva+size]
        raise ValueError(hex(address))

    def load(self, uc):
        uc.mem_map(self.base, (self.size+4095) & ~4095)
        uc.mem_write(self.base, self.data[:self.header_size])
        for rva, size, offset, _ in self.sections:
            if size: uc.mem_write(self.base+rva, self.data[offset:offset+size])


ORIGINAL = Pe((GAME/'AA.exe').read_bytes())
ACCEPTED = Pe((GAME/'reconstruction/native-game/runtime-expanded-v14/AA.exe').read_bytes())
assert sha(ORIGINAL.data) == ORIGINAL_SHA
assert sha(ACCEPTED.data) == ACCEPTED_SHA
for start, end in RANGES.values(): assert ORIGINAL.at(start, end-start) == ACCEPTED.at(start, end-start)


class Machine:
    KINGDOM = 0x2000000
    ARRAYS = {'raw_provided': 0x2010000, 'effective_provided': 0x2011000,
              'consumed': 0x2012000, 'stockpiled': 0x2013000}
    VECTOR = 0x2014000
    VECTOR_OWNER = 0x2015000
    STACK = 0x3008000
    STOP = 0x300f000

    def __init__(self, catalog, state, limits, cw):
        self.uc = uc = Uc(UC_ARCH_X86, UC_MODE_32)
        ORIGINAL.load(uc)
        uc.mem_map(0, 0x1000)  # isolated SEH head for flow prologue
        uc.mem_map(0x2000000, 0x100000)
        uc.mem_map(0x3000000, 0x10000)
        self.count = len(catalog['resources'])
        self.trace = []
        self.pending = None
        self.heap = 0x2080000
        for field, offset in [('raw_provided', 0x1ac), ('effective_provided', 0x1a0), ('consumed', 0x1b8), ('stockpiled', 0x1c4)]:
            self.put(self.KINGDOM+offset, self.ARRAYS[field]); self.vector(self.ARRAYS[field], state[field])
            self.put(self.KINGDOM+offset+4, self.count)
        registry = 0x2020000
        table = 0x2021000
        self.put(0xb36530, registry); self.put(registry+0x34, table); self.put(registry+0x38, self.count)
        manager = 0x2022000
        self.put(0xb3a660, manager); self.put(manager+0x28, self.count)
        for i, definition in enumerate(catalog['resources']):
            address = 0x2030000+i*0x100
            costs = 0x2040000+i*0x100
            self.put(table+i*4, address)
            self.put(address+0x18, {'Stockpiled':0,'Upkeep':1,'LimitedConsumed':2}[definition['kind']])
            self.put(address+0x1c, costs); self.vector(costs, definition['purchase_cost'])
            uc.mem_write(address+0x2c, bytes([definition['limited_provider']]))
        cache = 0x2050000
        self.put(0xb39ab0, cache)
        self.put(cache+0x1b4, 0x2051000); self.put(0x2051000, limits['provided_maximum'])
        self.put(cache+0x1b8, 0x2051004); self.put(0x2051004, limits['stockpiled_maximum'])
        self.put(self.VECTOR_OWNER, self.VECTOR)
        uc.reg_write(UC_X86_REG_FPCW, cw)
        self.context = uc.context_save()
        uc.hook_add(UC_HOOK_CODE, self.observe)

    def put(self, address, value): self.uc.mem_write(address, struct.pack('<I', value & 0xffffffff))
    def read(self, address): return struct.unpack('<I', self.uc.mem_read(address, 4))[0]
    def vector(self, address, values):
        if values: self.uc.mem_write(address, struct.pack('<'+'I'*len(values), *values))
    def values(self, address):
        return list(struct.unpack('<'+'I'*self.count, self.uc.mem_read(address, self.count*4))) if self.count else []
    def state(self): return {field:self.values(address) for field,address in self.ARRAYS.items()}

    def observe(self, uc, address, _size, _data):
        if self.pending and address == self.pending[0]:
            self.trace.append({'add':self.pending[1], 'consumed':self.values(self.ARRAYS['consumed'])})
            self.pending = None
        if address == 0x4e1f60:
            esp = uc.reg_read(UC_X86_REG_ESP)
            self.pending = (self.read(esp), bool(self.read(esp+4)&255))
        if address == 0x6f7810:
            esp = uc.reg_read(UC_X86_REG_ESP)
            size = self.read(esp+4)
            assert size == self.count*4
            result = self.heap
            self.heap += max(16, (size+15)&~15)
            assert self.heap < 0x20f0000
            uc.reg_write(UC_X86_REG_EAX, result)
            uc.reg_write(UC_X86_REG_ESP, esp+4)
            uc.reg_write(UC_X86_REG_EIP, self.read(esp))

    def call(self, address, args, this=None, stop=None):
        self.uc.context_restore(self.context)
        self.trace = []; self.pending = None
        self.put(self.STACK, self.STOP)
        for i,value in enumerate(args): self.put(self.STACK+4+i*4, value)
        self.uc.reg_write(UC_X86_REG_ESP, self.STACK)
        self.uc.reg_write(UC_X86_REG_ECX, self.KINGDOM if this is None else this)
        preserved = {reg:0x77770000+i for i,reg in enumerate([UC_X86_REG_EBX, UC_X86_REG_ESI, UC_X86_REG_EDI, UC_X86_REG_EBP])}
        for reg,value in preserved.items(): self.uc.reg_write(reg,value)
        self.uc.emu_start(address, stop or self.STOP, count=300000)
        assert self.uc.reg_read(UC_X86_REG_EIP) == (stop or self.STOP)
        if stop is None:
            # The emulator stops before the final address hook, so materialize
            # the helper-return observation if the helper was itself the call.
            if self.pending and self.pending[0] == self.STOP:
                self.trace.append({'add':self.pending[1], 'consumed':self.values(self.ARRAYS['consumed'])}); self.pending=None
            assert self.uc.reg_read(UC_X86_REG_ESP) == self.STACK+4+4*len(args)
            for reg,value in preserved.items(): assert self.uc.reg_read(reg) == value

    def action(self, action):
        kind = action['kind']
        if 'values' in action: self.vector(self.VECTOR, action['values'])
        if kind == 'Provider': self.call(0x4e3940, [action['index'], action['delta']])
        elif kind == 'Providers': self.call(0x4e4930, [self.VECTOR_OWNER, action['add']])
        elif kind == 'Consumption': self.call(0x4e48c0, [self.VECTOR_OWNER, action['add']])
        elif kind == 'Credit': self.call(0x4e39d0, [self.VECTOR_OWNER])
        elif kind == 'Debit': self.call(0x4e1690, [self.VECTOR_OWNER])
        elif kind == 'Afford': self.call(0x4e20a0, [self.VECTOR_OWNER])
        elif kind == 'Flow':
            self.put(self.KINGDOM+0x190, int(action['suppress_overdrawn']))
            self.call(0x4e9c40, [action['seconds']], this=self.KINGDOM+0x180, stop=0x4e9da6)
        else: raise AssertionError(kind)
        result = {'state':self.state(), 'trace':{'shortage_steps':self.trace}}
        if kind == 'Afford': result['affordable'] = bool(self.uc.reg_read(UC_X86_REG_EAX)&255)
        if kind == 'Flow':
            esp = self.uc.reg_read(UC_X86_REG_ESP)
            result['flow'] = {'minute_fraction':self.read(esp+0x10),
                'gross_production':self.values(self.read(esp+0x14)),
                'net_change':self.values(self.read(esp+0x20)),
                'stock_went_negative':bool(self.uc.reg_read(UC_X86_REG_EBX)&255)}
        return result


def catalog(kinds, costs, limited=()):
    return {'resources':[{'id':f'resource_{i}', 'kind':kind, 'limited_provider':i in limited,
                         'purchase_cost':[bits(v) for v in costs[i]]} for i,kind in enumerate(kinds)]}


def scenario(name, catalog_value, raw, base_consumed, stock, actions, maximum=100, stock_max=10000):
    count = len(raw)
    limits = {'provided_maximum':maximum, 'stockpiled_maximum':bits(stock_max)}
    state = {'raw_provided':list(map(bits,raw)), 'effective_provided':[bits(min(x,maximum) if catalog_value['resources'][i]['limited_provider'] else x) for i,x in enumerate(raw)],
             'consumed':list(map(bits,base_consumed)), 'stockpiled':list(map(bits,stock))}
    results=[]
    for mode,cw in MODES:
        machine=Machine(catalog_value,state,limits,cw)
        machine.call(0x4e1f60,[1])
        initial=machine.state()
        steps=[{'action':action, 'expected':machine.action(action)} for action in actions]
        results.append({'name':name, 'precision':mode, 'catalog':catalog_value, 'limits':limits, 'initial':initial, 'steps':steps})
    return results


def main():
    cases=[]
    stock='Stockpiled';upkeep='Upkeep';limited='LimitedConsumed'
    c=catalog([stock,upkeep,upkeep,upkeep,limited,upkeep,limited],
              [[0]*7,[2,0,0,0,0,0,0],[3,0,0,0,0,0,0],[0]*7,[0]*7,[0]*7,[0]*7],limited=(3,5))
    c['resources'][0]['id']='money';c['resources'][1]['id']='ammo';c['resources'][2]['id']='oil'
    def vector(kind,values,add=True): return {'kind':kind,'values':list(map(bits,values)),**({'add':add} if kind in ('Providers','Consumption') else {})}
    actions=[{'kind':'Provider','index':1,'delta':bits(3)},
             vector('Consumption',[1,7,2,0,1,0,1]),
             vector('Providers',[15,8,20,105,0,101,0]),
             vector('Providers',[1,4,2,10,0,5,0],False),
             vector('Consumption',[.1,1,1,0,1,0,1],False),
             {'kind':'Flow','seconds':bits(.0625),'suppress_overdrawn':False},
             vector('Afford',[200,9999,9999,0,0,0,0]),
             vector('Debit',[250,1,0,0,0,0,0]),
             {'kind':'Flow','seconds':bits(60),'suppress_overdrawn':False},
             vector('Credit',[1e6,-1e6,2,0,0,0,0])]
    cases += scenario('ammo_oil_shortages_and_expanded_caps',c,[20,10,4,20,0,20,0],[3,15,8,0,12,0,6],[100,0,0,0,0,0,0],actions)
    two=catalog([stock,upkeep],[[0,0],[.1,0]],limited=(1,))
    cases += scenario('stock_cap_negative_zero_and_underflow',two,[-0.0,99.0],[0.0,100.0],[-0.0,5.0],
       [{'kind':'Provider','index':1,'delta':bits(2)},vector('Consumption',[0,0]),
        {'kind':'Flow','seconds':1,'suppress_overdrawn':False},vector('Credit',[-0.0,-100]),
        {'kind':'Flow','seconds':bits(1.0),'suppress_overdrawn':True},vector('Afford',[0,1e30]),vector('Debit',[1,1])],stock_max=20)
    midpoint=catalog([stock,upkeep],[[0,0],[f32(0x3f800001),0]])
    cases += scenario('double_rounding_boundary',midpoint,[0,0],[-2.0**-60,1.5],[0,0],
        [vector('Consumption',[0,0]),{'kind':'Provider','index':1,'delta':bits(.25)},
         {'kind':'Flow','seconds':bits(.1),'suppress_overdrawn':False}])
    rng=random.Random(0x4e1f60)
    for i in range(40):
        coeffs=[.1,.2,.3,2,3,7.999999,1e-12,0,1.0000001192092896]
        costs=[[0]*4,[rng.choice(coeffs),0,rng.choice(coeffs),0],[0]*4,[rng.choice(coeffs),0,rng.choice(coeffs),0]]
        c2=catalog([stock,upkeep,stock,upkeep],costs,limited=(3,))
        raw=[rng.uniform(-1,10000),rng.uniform(0,120),rng.uniform(-1,20000),rng.uniform(0,120)]
        consumed=[rng.uniform(-10,1000),rng.uniform(0,120),rng.uniform(-10,1000),rng.uniform(0,120)]
        stocks=[rng.uniform(-1,1e4),0,rng.uniform(-1,1e4),0]
        actions=[vector('Consumption',[rng.uniform(-3,3) for _ in range(4)],i%2==0),
                 vector('Providers',[rng.uniform(-3,3) for _ in range(4)],i%3==0),
                 {'kind':'Provider','index':3,'delta':bits(rng.uniform(-150,150))},
                 {'kind':'Flow','seconds':bits(rng.choice([.0625,.1,1,60,-.1])),'suppress_overdrawn':i%2==0},
                 vector('Afford',[rng.uniform(0,2e4) for _ in range(4)]),
                 vector('Debit',[rng.uniform(0,2e4) for _ in range(4)]),
                 vector('Credit',[rng.uniform(-1e4,2e4) for _ in range(4)])]
        cases += scenario(f'ordered_precision_{i}',c2,raw,consumed,stocks,actions,maximum=100 if i%2 else 20)
    # Finite IEEE edge values, while staying inside the port's finite-result domain.
    edge=catalog([stock,upkeep,upkeep],[[0,0,0],[1,0,0],[.1,0,0]])
    for i, raw_bits in enumerate([1,0x00800000,0x3f800001,0x4b800001,0x60800001]):
        cases += scenario(f'finite_edge_{i}',edge,[f32(raw_bits),0,0],[0,f32(raw_bits),f32(raw_bits)],[1,0,0],
             [vector('Consumption',[0,0,0]),{'kind':'Provider','index':1,'delta':raw_bits},
              {'kind':'Flow','seconds':bits(.0625),'suppress_overdrawn':False}])
    cases += scenario('signed_integer_cap_comparison',catalog([stock,upkeep],[[0,0],[1,0]],limited=(1,)),
        [0,0],[0,0],[0,0],[{'kind':'Provider','index':1,'delta':bits(16777218)},
        {'kind':'Provider','index':1,'delta':bits(-2)}],maximum=16777217)
    cases += scenario('empty_registry',catalog([],[]),[],[],[],[vector('Providers',[]),vector('Consumption',[]),vector('Afford',[])])
    ranges={name:{'start_va':hex(a),'end_va_exclusive':hex(z),'sha256':sha(ORIGINAL.at(a,z-a)),
                  'unchanged_in_accepted_v14':True} for name,(a,z) in RANGES.items()}
    result={'schema':1,'passed':True,'original_sha256':ORIGINAL_SHA,'accepted_v14_sha256':ACCEPTED_SHA,
            'generator_sha256':sha(Path(__file__).read_bytes()),'ranges':ranges,'cases':cases,
            'method':'Unchanged original x86; real coupled arrays and shortage helper; only flow temporary allocator mocked; no arithmetic/decision hooks.',
            'precision_scope':'Three explicit nearest-even x87 precision modes; no assertion that the complete original game always uses any one mode.',
            'excluded':['Nonfinite input/results and alternate rounding modes','Bankruptcy ratios, event dispatch, attrition and downstream 4E9C40 calls','Scheduling, native registry-order construction, actor production rules and full gameplay parity']}
    out=ROOT/'crates/aa-economy/tests/fixtures/economy-native-vectors.json';out.parent.mkdir(parents=True,exist_ok=True)
    out.write_text(json.dumps(result,indent=2)+'\n')
    md=Cs(CS_ARCH_X86,CS_MODE_32)
    assembly=[]
    for name,(a,z) in RANGES.items():
        if name in ('zero','seconds_to_minutes'): continue
        assembly += [f'\n{name} {a:08x}..{z:08x}']
        assembly += [f'{i.address:08x} {i.bytes.hex():20} {i.mnemonic} {i.op_str}' for i in md.disasm(ORIGINAL.at(a,z-a),a)]
    (Path(__file__).parent/'economy-phase2-assembly.txt').write_text('\n'.join(assembly)+'\n')
    print(json.dumps({'path':str(out),'sha256':sha(out.read_bytes()),'scenarios':len(cases),'actions':sum(len(c['steps']) for c in cases)},indent=2))


if __name__=='__main__': main()

"""Native579A00/5797E0 on captured saved-component fields.

The actual saved-field captures are inputs to unchanged original/v14 routines.
RNG85D840 executes unchanged against the captured GWorld+D4 state. The current
unbound Mind+114 implementation is an explicit interruption before its body.
Absent Element is a native page-zero fault; Rust's required-Element refusal is
compared only for retained stores, not exception or malformed-pointer parity.
"""
from pathlib import Path
import json,sys,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
from native_formation import FormationMachine
from harness import P,V,PRECISION,NullDereference,VISITED,STUBS

class RequiredMind(Exception):pass

class SavedLeader(FormationMachine):
    def __init__(self,row,image):
        c=row['input'];self.captured=c
        actors={a['id']:a for a in c['actors']}
        form=c['formation']
        case={'precision':row['precision'],'operation':'set_leader','draws':[],
            'organization':{'owner_id':c['owner'],'owner_has_mind':c['owner_mind'],
                'facing':c['owner_facing'],'center':[0,0],
                'slot_count':len(c['members']),'primary_slot':c['primary_slot'],'groups':[],
                'members':[actors[i] if i else None for i in c['members']],
                'leader':actors[c['leader']] if c['leader'] else None,
                'formation':None if form is None else {'oriented':form['oriented'],'offsets':form['offsets']},
                'slot_positions':c['offsets']}}
        super().__init__(case)
        if image=='v14':
            bindings={address:self.r32(address) for address in (0xb36534,0xb3655c,0xb36560,0xb39af8)}
            direction_object=bytes(self.u.mem_read(0xb39a90,0x14))
            old=(P.image_size+4095)&~4095;new=(V.image_size+4095)&~4095
            if new>old:self.u.mem_map(P.base+old,new-old)
            self.u.mem_write(V.base,V.b[:0x1000])
            for vs,r,raw,off in V.sections:
                if raw:self.u.mem_write(V.base+r,V.b[off:off+raw])
            for address,value in bindings.items():self.w32(address,value)
            self.u.mem_write(0xb39a90,direction_object)
        self.requested=self.actor(actors[c['requested']]) if c['requested'] else 0
        self.rng=self.alloc(4);self.w32(self.rng,c['rng_state'])
        self.seams.pop(0x85d840)
        # Native4F02B0 reads GWorld+D4, whose loaded state is authoritative.
        world=self.r32(0xb36534);self.w32(world+0xd4,c['rng_state'])
        self.rng=world+0xd4
        for address in list(self.seams):
            if STUBS<=address<STUBS+0x1000:
                self.seams[address]=lambda m:(_ for _ in ()).throw(RequiredMind())
    def actor(self,member):
        key=member['id']
        if key in self.actors:return self.actors[key]
        actor=self.alloc(0x200);self.w32(actor+0x14,key);self.w32(actor+0x20,*member['position'])
        if member['element'] is not None:
            e=member['element'];assert e['owner']==key
            element=self.alloc(0x40);self.w32(actor+0x80,element)
            self.w32(element+4,actor);self.w32(element+0x18,e['slot']);self.w8(element+0x1c,e['attached'])
        self.actors[key]=actor;return actor
    def run(self):
        outcome='ok'
        try:self.call(0x579a00,self.org,[self.requested])
        except RequiredMind:outcome='missing_mind_execution'
        except NullDereference as error:
            assert str(error)=='0x18',f'unexpected native null fault: {error}'
            outcome='missing_element'
        leader=self.r32(self.org+0x40);ids={v:k for k,v in self.actors.items()}
        return {'outcome':outcome,'leader':ids.get(leader) if leader else 0,
            'offsets':[self.r32(self.positions+8*i,2) for i in range(self.n)],
            'rng_state':self.r32(self.rng),
            'messages':[[0x404f90,0xab0] for event in self.events if event['name']=='DetachedDiagnostic']}

def sha(b):return hashlib.sha256(b).hexdigest()
def main():
    inputs=HERE/'actual-saved-inputs.json';rows=[]
    for row in json.loads(inputs.read_text()):
        for image in ('original','v14'):
            m=SavedLeader(row,image)
            rows.append({'image':image,'precision':row['precision'],'input':row['input'],'output':m.run()})
    doc={'schema':1,'scope':__doc__,'original_sha256':sha(P.b),'v14_sha256':sha(V.b),
        'generator_sha256':sha(Path(__file__).read_bytes()),'captured_inputs_sha256':sha(inputs.read_bytes()),
        'executed_original_v14_equal':len(VISITED),
        'instructions':[{'va':hex(address),'bytes':raw} for address,raw in sorted(VISITED.items())],
        'cases':rows}
    output=HERE/'native-saved-leader.json';output.write_text(json.dumps(doc,indent=2)+'\n')
    print(json.dumps({'cases':len(rows),'sha256':sha(output.read_bytes())}))
if __name__=='__main__':main()

"""Read-only byte audit for Loading return4 and real UserReady registration.

These are static byte/source pins, not additional executed instructions.
The separate native_loading_return.py fixture owns execution counts.
"""
from pathlib import Path
import hashlib,json,sys

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h

def literal(image,address):
    result=bytearray()
    while True:
        pair=image.range(address+len(result),2)
        if pair==b'\0\0':return result.decode('utf-16le')
        result.extend(pair)

windows={
    'game_node_vtable':(0xA56C00,0x2C),
    'game_node_constructor':(0x608C60,0x20),
    'game_node_enter_jump':(0x608C80,5),
    'game_node_leave_jump':(0x5E8900,5),
    'bare_return':(0x4B2830,1),
    'game_node_menu_finished_return':(0x943800,3),
    'application_node4_allocation':(0x40CBF6,0x2E),
    'user_ready_callback':(0x6E80F0,0x15),
    'loading_owner_getter':(0x40D970,7),
    'user_ready_registration':(0x6E883D,0x30),
    'ready_button_initial_hide':(0x6E8BD0,0x20),
}
pins=[]
for name,(address,length) in windows.items():
    old=h.P.range(address,length)
    assert old==h.V.range(address,length),(name,hex(address))
    pins.append(dict(name=name,address=hex(address),length=length,bytes=old.hex()))
words=[h.P.u32(0xA56C00+i)for i in range(0,0x2C,4)]
assert words[4:7]==[0x4B2830,0x608C80,0x5E8900]
assert words[10]==0x943800
literals={hex(a):literal(h.P,a)for a in [0xA7EF9C,0xA7F0A4]}
assert literals=={'0xa7ef9c':'UserReady','0xa7f0a4':'app_status_finished'}
assert all(literal(h.V,int(a,16))==s for a,s in literals.items())
doc=dict(schema=1,scope=__doc__,static_windows=pins,vtable_words=[hex(x)for x in words],literals=literals,
         producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
         qualification='No native process or gameplay execution; ordinary source callback registration only. Full HUD construction, loading display tail and actual frame readiness are outside this audit.')
raw=(json.dumps(doc,indent=2)+'\n').encode()
path=HERE/'loading-return-source-audit.json'
if '--write'in sys.argv:path.write_bytes(raw)
else:assert path.read_bytes()==raw
print(json.dumps(dict(static_windows=len(pins),executions=0,sha256=hashlib.sha256(raw).hexdigest())))

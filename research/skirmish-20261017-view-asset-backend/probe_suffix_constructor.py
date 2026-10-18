import sys,json
sys.path.insert(0,'research/skirmish-20261017-svar-field-order');import native_full_sources as f
m=f.order.Machine(b'','Double53');m.invoke(0x4013e0,0xb43b40,[]);s=m.alloc(0x80);m.invoke(0x4b2ce0,s,[]);m.invoke(0x401bd0,s,[]);print(json.dumps(f.snapshot(m,s)[50]))

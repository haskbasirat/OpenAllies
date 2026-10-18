"""Create a separate Sun observer from the sealed, successful point tracer."""
from pathlib import Path
import hashlib

HERE = Path(__file__).resolve().parent
BASE = HERE.parent / 'skirmish-20261016-load-world-globals/capture_startup_globals.py'

def main():
    source = BASE.read_text()
    # Every original capture remains immutable; this file writes a new script.
    replacements = {
        "parser.add_argument('--words',type=int,nargs=4,required=True)": "parser.add_argument('--words',type=int,nargs=3,default=[0,1,2])",
        "assert len(set(args.words))==4 and all(0<=i<6 for i in args.words)": "assert sorted(args.words)==[0,1,2]",
        "watch_addresses=[0xb3a8c4+4*i for i in args.words]": "watch_addresses=[0xb2814c+4*i for i in args.words]",
        "'point_words':list(struct.unpack('<6I',memory(0xb3a8c4,24)))": "'point_words':list(struct.unpack('<6I',memory(0xb3a8c4,24))), 'sun_words':list(struct.unpack('<3I',memory(0xb2814c,12)))",
        "for i in range(4):c.Dr7": "for i in range(3):c.Dr7",
        "for i in range(4)]==watch_addresses": "for i in range(3)]==watch_addresses",
        "(0x4e3e90,'global_point_getter')": "(0x4e3e90,'global_point_getter'),(0x4ee0a0,'sun_ctor_entry')",
        "'change_state_phase_stored','global_point_getter']": "'change_state_phase_stored','global_point_getter','sun_ctor_entry','sun_ctor_return']",
        "if label=='session_ctor_entry':breakpoint(word(c.Esp),'session_ctor_return')": "if label=='session_ctor_entry':breakpoint(word(c.Esp),'session_ctor_return')\n                        if label=='sun_ctor_entry':breakpoint(word(c.Esp),'sun_ctor_return')",
        "if probe_state['index']==3:": "if probe_state['index']==2:",
        "assert list(struct.unpack('<6I',memory(0xb3a8c4,24)))==observation_cutoff['point_words']": "assert list(struct.unpack('<6I',memory(0xb3a8c4,24)))==observation_cutoff['point_words']\n                            assert list(struct.unpack('<3I',memory(0xb2814c,12)))==observation_cutoff['sun_words']",
        "'observed_native_point_write'": "'observed_native_sun_write'",
        "range(4) if hits": "range(3) if hits",
        "len(probes)!=4": "len(probes)!=3",
        "len(probes)==4": "len(probes)==3",
        "source+Session+four isolated": "source+Session+three isolated",
        "'observed_native_point_writes':watch_writes": "'observed_native_sun_writes':watch_writes",
        "no perpetual point immutability claim.": "no perpetual Sun immutability or Sun constructor observation claim unless that breakpoint is recorded.",
    }
    for old, new in replacements.items():
        assert old in source, old
        source = source.replace(old, new)
    source = source.replace('Four data watches cover explicitly selected 32-bit words.', 'Three data watches cover the12 Sun startup bytes at B2814C..54.')
    source = source.replace('Trace actual startup point-shape writes', 'Trace actual Sun startup-global writes')
    source = source.replace('same-value isolated MOV probes calibrate every', 'same-value isolated MOV probes calibrate every')
    output = HERE / 'capture_sun_startup.py'
    assert not output.exists()
    output.write_text(source)
    print({'base_sha256': hashlib.sha256(BASE.read_bytes()).hexdigest(), 'new_sha256': hashlib.sha256(output.read_bytes()).hexdigest()})

if __name__ == '__main__':
    main()

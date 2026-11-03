KKC_GGroup arithmetic: the original's code under Unicorn at PC53.

Producer: native_group.py (--write creates, without arguments replays and checks the file is
byte-identical; the executed bytes are checked equal in both pinned images). Output:
native-group.json. Replayed by the unit tests of crates/aa-play/src/orders/group.rs.

Proven, every value to the bit:

  0x4DD590   the robust centre of a list of things (center)
  0x4DD860   a thing's offset from a centre, near or on the edge of the range (offset)
  0x4DF600   the places of a group: the formation's own, the others where they stand, less
             their mean; a member that does not resolve keeps its place and counts in the
             divisor (place)
  0x4DE020   a member's place at a point; the tile walk 0x4DD030 / 0x4DDDE0 / 0x4DD150 back to
             the point, tile by tile, when the place is outside the zone (offset_at; the tiles
             tested are compared too)
  0x869D30 + 0x4DDF30 + 0x4D0780 + 0x579A60 + 0x579980
             where 0x4E0940 inserts a thing in the group's list: an organization's +320 when
             positive, else its element's +2C4, an element's +2C4, anything else 0; highest
             first, ties after (order)
  0x4E0330   the commanded Update from its busy test on: each member's time, the longest, and
             what vt+88 is told (speed)

Seams: 0x4F2420 (in the zone; the case's circle), 0x5581B0 (the walk's constructor),
0x4DEB80, 0x4DDB60, 0x52CE70, 0x515F40, 0x4B2E60, the Mind's vt+44, +48, +64, +88 and the
group's reschedule in the speed cases. Nothing else is stubbed in those functions.

Not proven here, compared with the code only: the idle and under-attack Updates 0x4DFFF0 and
0x4E00A0, the delivery 0x4DF840 / 0x4DE7F0, the zone flood behind 0x4F2420 (group.rs has a circle
over open tiles) and a lone actor as a group member in the speed loop (no host: group.rs evens
organizations only).

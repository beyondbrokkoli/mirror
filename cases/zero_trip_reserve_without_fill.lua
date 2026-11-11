-- EXPECT: 5
-- EXPECT: 0	0	0	0	0
-- EXPECT: 1
-- The P10 law (battery pin of probes/p10_reserve_without_fill.lua):
-- the pre-header reserve is minted from static loop shape, so it fires
-- even when the guard fails on its first test (`for i = 5, 5` is
-- zero-trip under the exclusive default) — size without fill. The
-- border rises to the latched bound 5 with zero stores, every read in
-- the reserved span answers the zero lane (in-span, no growth), and
-- the table is the one allocation. This is the buffer property the
-- memory-lane first cut builds on; the inclusive twin is pinned by
-- for_inclusive_zero_trip_reserve.lua.
local t = {}
local n = 5
for i = 5, n do
  t[i] = i
end
print(#t)
print(t[0], t[1], t[2], t[3], t[4])
print(sys_alloc_count())

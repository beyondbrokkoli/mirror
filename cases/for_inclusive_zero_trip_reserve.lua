-- FLAGS: --for-limit=inclusive
-- EXPECT: 6
-- EXPECT: 0
-- EXPECT: 1
-- The inclusive zero-trip reserve (battery pin of
-- probes/p15_inclusive_zero_trip_reserve.lua, extends the P10 law): a
-- zero-trip loop needs start > limit (`for i = 6, 5`); the pre-header
-- reserve still fires from static shape and the latched bound raises
-- the border to limit + 1 = 6 — one cell over the exclusive reserve —
-- with zero stores. Not a leak: the same latched bound a one-trip
-- `for i = 5, 5` would size, and the off-by-one ledger stays legible.
local t = {}
for i = 6, 5 do
  t[i] = i
end
print(#t)
print(t[5])
print(sys_alloc_count())

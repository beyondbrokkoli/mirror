-- EXPECT: 5
-- EXPECT: 1
-- The P16 law, default quadrant (battery pin of
-- probes/p16_row_housing_zero_trip.lua): row housing changes nothing
-- in the zero-trip reserve arithmetic — the tight border stays
-- raw_latched_bound + K_eff == 5, the decl ctor mints contains_tables
-- statically before any store, and the root free walks the committed
-- buffer where every zero-lane slot fails is_valid_ptr and is skipped,
-- never dereferenced. Zero trips mean zero rows: the reserve
-- pre-commits no row, so sys_alloc_count() == 1 (the parent only).
-- The base-1 and inclusive quadrants (borders 4 and 6) live in the
-- probe header; the base-1 inclusive cancellation twins are pinned by
-- for_inclusive_base1_zero_trip_row_borders.lua.
local t = {}
for i = 6, 5 do
  t[i] = { i }
end
print(#t)
print(sys_alloc_count())

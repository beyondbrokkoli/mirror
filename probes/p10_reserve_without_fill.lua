-- P10 reserve semantics: the compiler-minted pre-header reserve with a
-- zero-trip loop, so the table is reserved and never filled. The plan
-- is minted from static loop shape, so the reserve fires even though
-- the guard fails on its first test.
-- Run: lua run.lua probe probes/p10_reserve_without_fill.lua
-- Verdicts: #t == 5 with zero stores (border raised by reserve alone);
-- reads t[0]..t[4] answer the zero lane (in-span, no growth);
-- sys_alloc_count() == 1; the runtime sidecar shows RT_ALLOC and the
-- root RT_FREE only (no reserve/grow null tells, slots 157/158). This
-- is the buffer property the memory-lane first cut relies on: size
-- without fill, valid zero reads, one allocation.

local t = {}
local n = 5
for i = 5, n do
  t[i] = i
end
print(#t)
print(t[0], t[1], t[2], t[3], t[4])
print(sys_alloc_count())

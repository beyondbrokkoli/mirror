-- P15 the zero-trip reserve under inclusive (extends p10): a
-- zero-trip inclusive loop needs start > limit (`for i = 6, 5`);
-- the pre-header reserve still fires from static shape and the
-- latched bound raises the border to limit + 1 = 6 — one cell over
-- p10's exclusive 5, with zero stores. The extra border cell is not
-- a leak: it is the same latched bound a 1-trip `for i = 5, 5`
-- would size, and the off-by-one ledger stays legible.
-- Run: lua run.lua probe probes/p15_inclusive_zero_trip_reserve.lua
-- Verdicts: #t == 6 with zero stores; t[5] answers the zero lane
-- (in-span, no growth); sys_alloc_count() == 1.

local t = {}
do
  local i = 6
  local limit = 5 + 1
  while i < limit do
    t[i] = i
    i = i + 1
  end
end
print(#t)
print(t[5])
print(sys_alloc_count())

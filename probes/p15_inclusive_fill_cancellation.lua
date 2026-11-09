-- P15 the reserve cancellation: the hand-expanded physical picture of
-- `--index-base=1 --for-limit=inclusive` over `for i = 1, n do
-- t[i] = i end` — user keys 1..n translate to physical i - 1, the
-- latch is n + 1, so the tight reserve is (n + 1) + (-1) = n and #t
-- equals n. The +1 of the inclusive latch and the -1 of the 1-based
-- key cancel in the arithmetic, never in special-case code. Real-Lua
-- ground truth: #t == 5, t[1]..t[5] == 1 2 3 4 5.
-- Run: lua run.lua probe probes/p15_inclusive_fill_cancellation.lua
-- Verdicts: #t == 5; the five reads print 1 2 3 4 5; sys_alloc_count()
-- == 1 (the reserve is exact — no grow).

local t = {}
local n = 5
do
  local i = 1
  local limit = n + 1
  while i < limit do
    t[i - 1] = i
    i = i + 1
  end
end
print(#t)
print(t[0], t[1], t[2], t[3], t[4])
print(sys_alloc_count())

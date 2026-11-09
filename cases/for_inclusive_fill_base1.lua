-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 5
-- EXPECT: 1	2	3	4	5
-- EXPECT: 1
-- The cancellation: the +1 of the inclusive latch and the -1 of the
-- 1-based key land in one reserve arithmetic — (n + 1) + (-1) = n —
-- with no per-combo casing. #t equals the logical length n and real
-- Lua agrees (`for i = 1, n do t[i] = i end` gives #t == n). The
-- guard shape and the guard-offset fold are untouched: the base and
-- the limit never share emission math.
local t = {}
local n = 5
for i = 1, n do
  t[i] = i
end
print(#t)
print(t[1], t[2], t[3], t[4], t[5])
print(sys_alloc_count())

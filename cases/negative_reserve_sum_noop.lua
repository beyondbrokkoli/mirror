-- EXPECT: 0
-- EXPECT: 0	0
-- EXPECT: 1
-- The P14 law (battery pin of probes/p14_negative_reserve_sum.lua): a
-- zero-trip loop whose admitted offset store makes the tight reserve
-- negative. `t[i - 5]` is GuardOffset(-4) with entry_const 5 (5 - 4 =
-- 1 >= 0, admitted, K_eff = -4), and the guard `for i = 5, 2` never
-- trips, so the widened bound 2 + (-4) = -2 reaches the runtime as the
-- checked sum's operands — a no-op by the two growth guards (want <=
-- len early-returns; the span floor is 8), never a clamp, never an
-- under-allocation. #t == 0, zero reads answered, one allocation.
local t = {}
for i = 5, 2 do
  t[i - 5] = i
end
print(#t)
print(t[0], t[1])
print(sys_alloc_count())

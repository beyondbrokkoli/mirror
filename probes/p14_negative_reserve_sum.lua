-- P14 negative reserve sum: a zero-trip loop whose admitted offset key
-- makes the tight reserve negative. Guard i = 5 never runs (exclusive
-- bound 2), but the store t[i - 5] is GuardOffset(-4) with entry_const
-- 5 (5 - 4 = 1 >= 0), so it is admitted and contributes K_eff = -4.
-- Run: lua run.lua probe probes/p14_negative_reserve_sum.lua
-- Before the tight bound (today): widen clamps at max(K, 0), the
-- reserve is the raw bound 2, so #t == 2 with zero stores.
-- After the tight bound (raw_bound + K_eff): the reserve computes
-- 2 + (-4) = -2, a runtime no-op by two guards (the border max at
-- rt.rs:267 never fires for n <= border; span growth early-returns on
-- want <= len at rt.rs:656 and floors at 8 at rt.rs:659), so #t == 0.
-- Both phases: no death, no growth, one allocation, zero reads.

local t = {}
for i = 5, 2 do
  t[i - 5] = i
end
print(#t)
print(t[0], t[1])
print(sys_alloc_count())

-- FLAGS: --index-base=1
-- EXPECT: 9
-- EXPECT: 1	8
-- A dynamic start declines the claim and keeps the raw bound: the
-- guard is provably nonneg through its Var chain (start holds Lit 1),
-- so detect_fill_loop mints the plan, but the admission gate needs
-- the entry VALUE (entry - 1 >= 0) and the lane's constant knowledge
-- only records Integer inits -- entry_const is None, the store rides
-- the checked arm, key_offset stays 0, and the reserve covers the raw
-- bound 9. Every value still lands (the checked arm translates each
-- key once), but the border over-reports by the un-folded offset: 9
-- against the golden literal spelling's 8 (fill_rides_fast_lane).
-- The declined-claim law extends zero_start_fill_checked_arm: a
-- declined store contributes no offset, forced (negative first key)
-- or by knowledge gap alike.
local start = 1
local t = {}
for i = start, 9 do
  t[i] = i
end
print(#t)
print(t[1], t[8])

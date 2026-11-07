-- FLAGS: --index-base=1
-- EXPECT: 8
-- EXPECT: 1	8
-- The Var chain's bound re-proves the entry: start holds Lit 1, so
-- detect_fill_loop mints the plan with entry bound 1 beside its own
-- nonneg proof, and the admission gate reads that bound (1 - 1 = 0)
-- where the lane's constant knowledge sees nothing -- the store rides
-- the inline fast lane with one sub in the key lane, and the tight
-- reserve covers bound - 1 = 8 cells, so the border equals the
-- logical length. Supersedes the audit's over-report pin
-- (index_base1_dynamic_start_raw_reserve): the declined-claim law
-- itself survives in index_base1_zero_start_fill_checked_arm (a
-- forced negative first key) and in the entry-no-bound corner.
local start = 1
local t = {}
for i = start, 9 do
  t[i] = i
end
print(#t)
print(t[1], t[8])

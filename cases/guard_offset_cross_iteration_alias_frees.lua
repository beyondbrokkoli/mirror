-- EXPECT: 20
-- EXPECT: 2
-- EXPECT: 0
-- The P23 law (battery pin of
-- probes/p23_guard_offset_cross_iteration_alias.lua): two distinct
-- key shapes over one guard — Guard and GuardOffset(+1) — alias across
-- iterations (trip i's offset row lands in the slot trip i + 1's guard
-- row overwrites), and the second writer must not lose the displaced
-- row + grandchild. Stdout cannot tell (final slot values agree either
-- way) — the ledger is the oracle: the first block repeats one shape
-- (the second sighting declines to the checked face where set_core's
-- replace-reclaim frees the displaced child), the second block mixes
-- the two shapes, and both end at sys_alloc_count() == 0 — the
-- gap-live counterfactual reads 4 with an atexit leak tell. The
-- backward twin t[i - 1] never reaches the lane at any base
-- (first + k_phys < 0 declines wholesale).
do
  local a = {}
  for i = 0, 3 do
    a[i] = { { i } }
    a[i] = { { i * 10 } }
  end
  print(a[2][0][0])
end
do
  local t = {}
  for i = 0, 3 do
    t[i] = { { i } }
    t[i + 1] = { { i * 10 } }
  end
  print(t[2][0][0])
end
print(sys_alloc_count())

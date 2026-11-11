-- FLAGS: --index-base=1
-- EXPECT: 4
-- EXPECT: 1
-- The F2 law (if boundary), probed 2026-11-11: a constant killed in
-- an if arm used to resurrect at the join — the ConstEnv's join read
-- a never-populated map, so the lane admitted entry_const = Some(1)
-- while the true entry is the arm's 0, physical -1, a store before
-- the span (exit 134, free(): invalid size on the unsound build).
-- Kills now persist and the join keeps a constant only when both
-- arms end at the same one: the entry falls to the plan's minted
-- bound 0, the admission 0 + (-1) < 0 declines, and the store rides
-- the checked arm — the raw-bound reserve over-reports #t by the
-- base shift (4 while user keys 1..3 are written), the
-- index_base1_stalemin_entry_declines behavior.
local t = {}
local i = 1
if 0 < 1 then
  i = 0
end
while i < 4 do
  t[i] = i
  i = i + 1
end
print(#t)
print(sys_alloc_count())

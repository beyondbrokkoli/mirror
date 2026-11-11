-- FLAGS: --index-base=1
-- EXPECT: 4
-- EXPECT: 1
-- The F2 law (do boundary), probed 2026-11-11: the do-block twin of
-- the if-arm case. A kill inside a block used to lift with the
-- block's end — the scan restored the whole declaration overlay
-- wholesale, resurrecting the outer binding's pre-block constant, so
-- the lane admitted entry_const = Some(1) while the true entry is 0
-- (the assignment wrote the outer binding and persists past the
-- block). Kills now persist: the entry falls to the plan's minted
-- bound 0, the admission declines, and the store rides the checked
-- arm with the raw-bound reserve over-report (4 while user keys 1..3
-- are written). Also probed at the fix: exit 134 on the unsound
-- build.
local t = {}
local i = 1
do
  i = 0
end
while i < 4 do
  t[i] = i
  i = i + 1
end
print(#t)
print(sys_alloc_count())

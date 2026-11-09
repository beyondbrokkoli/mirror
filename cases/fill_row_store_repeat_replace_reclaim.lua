-- EXPECT: 2
-- EXPECT: 5
-- The fast-lane claim law (post-P21 fix), base-0: two same-slot row
-- stores in one fill body. The first sighting rides the lane — its
-- slot is fresh — while the second sighting is a replace and keeps the
-- checked face, where set_core's replace arm frees the displaced row
-- (+ grandchild) before the clobber. The ledger closes at 5: parent +
-- 3 trips x (row + grandchild) - 2 displaced pairs (pre-fix: 9 with a
-- 4-slot leak tell at exit; the twin law receipt: the first store keeps
-- its inline fast mint and the repeat rides alm_tbl_set_row).
local t = {}
for i = 0, 2 do
  t[i] = { { i } }
  t[i] = { { i + 1 } }
end
print(#t)
print(sys_alloc_count())

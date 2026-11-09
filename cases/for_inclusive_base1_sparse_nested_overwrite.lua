-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 8
-- EXPECT: 0
-- EXPECT: 100
-- EXPECT: 400
-- EXPECT: 9
-- The displaced-row overwrite law (post-P21 fix), the battery pin of
-- P21 Q4: a contains_tables parent written twice at the same sparse
-- keys — pass 1's rows are every slot's first occupant, so pass 2's
-- stores are replaces, and set_core's replace arm frees all four
-- displaced rows (and their grandchildren) before the clobbering
-- store — the ledger closes at 9 (parent + 4 rows + 4 grandchildren)
-- with a clean zero-leak exit (pre-fix: 17 and a slot-90 leak tell at
-- exit, receipted in probes/p21_sparse_overwrite_displaced_row_leak.lua).
-- The stride keys decline the conversion battery, so every store rides
-- the generic face. The read pair pins the base-1 spelling law at two
-- depths: r[0][0] reads the past end lookalike (zero lane -> 0), and
-- r[1][1] is the pass-2 grandchild scalar.
local t = {}
for i = 1, 4 do
  t[i * 2] = { { i } }
end
for i = 1, 4 do
  t[i * 2] = { { i * 100 } }
end
print(#t)
local r = t[2]
print(r[0][0])
print(r[1][1])
print(t[8][1][1])
print(sys_alloc_count())

-- EXPECT: 3
-- EXPECT: 3
-- EXPECT: 11
-- EXPECT: 0
-- EXPECT: 4
-- The P20 shape on the clean face (battery pin of
-- probes/p20_refused_outer_null_host_fast_store.lua, base-0 exclusive
-- quadrant): a row ctor store admits to the fast lane and the inner
-- fill's stores admit too, so the whole body rides inline faces —
-- nothing refuses, nothing reclaims. The ledger: #t == 3 (three rows),
-- each row sized 3 by the ctor reserve borrowing the enclosing latch
-- bound, t[3][2] == 0 proves the row's unwritten cells answer the zero
-- lane, and sys_alloc_count() == 4 (1 parent + 3 rows). The base-1
-- twins (null_host_inner_fill_base1_*) pin the refused-outer decline.
local t = {}
for i = 0, 3 do
  t[i] = { i }
  for j = 1, 2 do
    t[i][j] = i * 10 + j
  end
end
print(#t)
print(#t[1])
print(t[1][1])
print(t[3][2])
print(sys_alloc_count())

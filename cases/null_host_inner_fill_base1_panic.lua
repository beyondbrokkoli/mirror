-- FLAGS: --index-base=1
-- EXPECT_PANIC: alm runtime error: attempt to index a nil value (null table row)
-- The P20 law, refused-outer quadrant (battery pin of
-- probes/p20_refused_outer_null_host_fast_store.lua, whose receipts
-- hold the SIGSEGV before-picture): under base-1 the first trip's
-- outer store t[0] lowers to physical key -1, set_core refuses and
-- reclaims the row, and the slot keeps its zero word. The inner fill's
-- host load then reads that zero lane as a null table row — and the
-- store's fast-lane eligibility declines to the checked face (the
-- c0156b0 walk fact), where the null-host guard aborts loud. The IR
-- pin: both stores ride alm_tbl_set_row / alm_tbl_set, no inline gep
-- mint between them. Loud decline, never the wild write.
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

-- P19 the sparse nested fill's border ledger (composes p12's row
-- housing with gap keys): stride keys (`i * 2`) sit outside
-- guard_key_shape's fold, so the loop is NOT a fill loop — no reserve
-- plan, no fast lane, no per-row ctor reserve — and each row store rides
-- the generic face while the border tracks the span high-water. The
-- nested rows give t's free walk real noncontiguous housing: row
-- children at 2, 4, 6 and unwritten gap words between them, all
-- recovered by the recursive contains_tables walk at the scope exit.
-- Run: lua run.lua probe probes/p19_sparse_nested_fill_border.lua
-- Quadrants:
--   target/release/alm                        -o /tmp/d_q1.so ... && /tmp/d_q1.so   -> 7, 1, 7
--   target/release/alm --index-base=1         -o /tmp/d_q2.so ... && /tmp/d_q2.so   -> 6, 0, 7
--   target/release/alm --for-limit=inclusive  -o /tmp/d_q3.so ... && /tmp/d_q3.so   -> 9, 1, 9
--   target/release/alm --index-base=1 --for-limit=inclusive -o /tmp/d_q4.so ... && /tmp/d_q4.so -> 8, 0, 9
-- Verdicts (all four quadrants exit 0):
--   Q1: trips 1..3, user keys 2, 4, 6, #t == 7 (span high-water: the gap
--   cells 1, 3, 5 count in the border and answer the zero lane on
--   reads); the row borrow `local r = t[2]` reads the claimed row
--   through its housing, and `r[0][0]` prints the grandchild's scalar
--   cell (1 — the inner ctor's first implicit elem sits at physical 0);
--   sys == 7 at print (parent + 3 rows + 3 grandchildren), 0 at exit.
--   Q3: trips 1..4, keys 2, 4, 6, 8, #t == 9, r[0][0] == 1, sys == 9.
--   Q2/Q4 (base 1): the user keys fold at the seam — element keys shift
--   to (key - 1) — so the same writes cover 1, 3, 5 (exclusive: #t 6,
--   sys 7) and 1, 3, 5, 7 (inclusive: #t 8, sys 9); t keeps its
--   physical span truth (the border holds max key + 1 == the count of
--   existing slots, the off-by-one ledger base-blind). The middle print
--   shifts too: under base 1 the inner read `r[0]` runs at user key 0
--   -> physical -1, answers the zero lane (a null row — a read, never a
--   store, so no null-row-store guard), and `r[0][0]` prints 0; the
--   base-1 spelling of the grandchild scalar is `r[1][0]`, which prints
--   1. The zero words in the gaps are skipped by free_tbl's is_valid_ptr
--   gate exactly as in p16 — no deref, no RT_FREE_CHILD on gaps, one
--   canonical rc 0.
-- IR pins (p19_*.ll): no reserve anywhere (the stride key declines
--   detect_fill_loop's collect_fills — outside the guard shape); t's
--   decl ctor still emits flags 138 (contains_tables from
--   stored_ctor_parents at the store site), the row ctor emits flags
--   0x80 for its inline grandchild, and the row store rides
--   alm_tbl_set_row with the computed key lowered as written.
-- Adjacent law note: alm's border is the physical span law, not Lua's
--   search-by-probe guess — for stride keys Real Lua reports 0
--   (t[1] is nil) while alm reports the span; the divergence is the
--   documented border law, not a defect (alm's border == the count of
--   existing slots, and the gap reads answer the zero lane).

local t = {}
for i = 1, 4 do
  t[i * 2] = { { i } }
end
print(#t)
local r = t[2]
print(r[0][0])
print(sys_alloc_count())

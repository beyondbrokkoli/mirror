-- P18 the refused row store under base 1 with a zero start (the
-- 5cd8af9 reclaim meets the per-row ctor reserve): under base 1 the user
-- key 0 is physical -1, the loop `for i = 0, 4` starts exactly there,
-- the fast lane declines the store for every trip (0 + (-1) < 0 — the
-- one admission law, zero start declines under base 1), and every store
-- rides the generic alm_tbl_set_row face. Trip 0's refuse reclaims that
-- trip's row in set_core; every later trip lands in domain.
-- Run: lua run.lua probe probes/p18_base1_inclusive_negative_start_reclaim.lua
-- Quadrants:
--   target/release/alm                                    -o /tmp/c_q1.so ... && /tmp/c_q1.so   -> #t 4, sys 5
--   target/release/alm --index-base=1                     -o /tmp/c_q2.so ... && /tmp/c_q2.so   -> #t 4, sys 4
--   target/release/alm --for-limit=inclusive              -o /tmp/c_q3.so ... && /tmp/c_q3.so   -> #t 5, sys 6
--   target/release/alm --index-base=1 --for-limit=inclusive -o /tmp/c_q4.so ... && /tmp/c_q4.so -> #t 5, sys 5
-- Verdicts (all four quadrants exit 0):
--   Q1 (head law): the exclusive latch is 4, the reserve rides it
--   unwidened (K_eff 0), the four rows land 0..3, #t == 4, sys == 5 at
--   print (parent + 4 rows), 0 at exit.
--   Q3 (inclusive): the latch is limit + 1 == 5, trips 0..4, five rows
--   0..4, #t == 5, sys == 6 at print (tight: the reserve == the span),
--   0 at exit.
--   Q2/Q4 (base 1, the fixed quadrants): trip 0's store arrives at
--   physical -1, is refused, and set_core reclaims that trip's row;
--   trips 1..3 / 1..4 land at physical 0..2 / 0..3. The abort this
--   probe demanded is FIXED by mint-order: the row's ctor reserve
--   (p12's borrow-the-bound reserve) is minted BEFORE the store faces
--   (`table_set_fast` / `table_set`), reserve-while-alive, with the
--   bound register minted in the pre-header and independent of the
--   store's outcome. The pre-fix .ll carried
--   `@alm_tbl_set_row(ptr %t, ...)` followed by
--   `@alm_tbl_reserve(ptr %row, i64 %reserve_reg)` on the same
--   register, so the reserve loaded the header set_core had just
--   poisoned (0xdeadf8eedeadf8ee, rc 134 fatal); the poison guard in
--   rt.rs stays untouched — it is the double-life tell, not a
--   refuse-safety hole. Post-fix the base-1 receipts EXPOSE the parked
--   law candidate below: #t rides the RAW latched bound (4 / 5), one
--   over the physical span (3 / 4), key_offset admissive-only; sys ==
--   the refuse-reclaim ledger, parent + rows - reclaimed (4 / 5), the
--   5cd8af9 law doing its job inside a live composition.
-- Second-order note (not a crash; the envelope family pinned by
--   index_base1_mixed_offset_envelope): at base-1 quadrants the
--   pre-header reserve rides the RAW latched bound, because key_offset
--   is fed only by ADMITTED stores and this loop admits nothing
--   (fast_lane's `first + k_phys >= 0` gate declines at -1). Widening
--   on the recognized key shape (the declined GuardOffset(-1)) would
--   land #t == 3 (exclusive, trips 1..3) / #t == 4 (inclusive, trips
--   1..4) — exactly the physical spans and real-Lua's border for the
--   landed keys. Parked as a law candidate: this file is the
--   case-demand and now carries its receipts, the relock would flip
--   only the base-1 negative-start reserve drift (4 -> 3 / 5 -> 4).

local t = {}
for i = 0, 4 do
  t[i] = { i }
end
print(#t)
print(sys_alloc_count())

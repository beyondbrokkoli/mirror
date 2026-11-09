-- P16 zero-trip row housing under the pre-header reserve (extends p15's
-- scalar pin to nested rows): the loop never trips (`for i = 6, 5` needs
-- start > limit under inclusive; under exclusive it also trips 0), but the
-- static mint is trip-blind: `t` rides the pre-header reserve plan (its
-- stores are guard-shaped, dense, call-free, break-free), the reserve
-- raises #t to the latched reserve, and the decl ctor mints
-- contains_tables at allocation time from stored_ctor_parents — the bit
-- is set before the loop runs, so free_tbl's recursive child walk covers
-- every unwritten zero-lane slot at the scope exit.
-- Run: lua run.lua probe probes/p16_row_housing_zero_trip.lua
-- Quadrants (the compiler directly; the probe runner has no per-file flag
-- slot):
--   target/release/alm -o /tmp/a_q1.so probes/p16_row_housing_zero_trip.lua && /tmp/a_q1.so
--   target/release/alm --index-base=1             -o /tmp/a_q2.so ... && /tmp/a_q2.so   -> #t 4, sys 1
--   target/release/alm --for-limit=inclusive      -o /tmp/a_q3.so ... && /tmp/a_q3.so   -> #t 6, sys 1
--   target/release/alm --index-base=1 --for-limit=inclusive -o /tmp/a_q4.so ... && /tmp/a_q4.so
-- Verdicts (all four quadrants exit 0):
--   #t == raw_latched_bound + K_eff: 5 / 4 / 6 / 5 (the tight form holds
--   for rows: the base-1 key offset and the inclusive latch cancel in the
--   arithmetic — 6 + (-1) == 5 — never in special-case code). One
--   canonical surface: the inclusive zero-trip border == limit + 1 == 6,
--   one cell over the exclusive pin, the same latched bound a one-trip
--   `for i = 5, 5` would size; the off-by-one ledger stays legible.
--   sys_alloc_count() == 1 at print (the parent only — zero trips mean
--   zero rows allocated, and the reserve pre-commits no rows); 0 at exit.
--   The root free runs free_tbl's child loop over each committed slot:
--   the ESIZE==8 dense scan (len * 8 == the committed buffer, in bounds)
--   feeds every word to the is_valid_ptr gate, and a zero word answers
--   `addr > 0x0100_0000` false — the zero-lane slots are skipped, never
--   dereferenced, with no RT_FREE_CHILD tell and no RT_GROW_NULL. The
--   zero word is the only reachable slot value: alloc_zeroed at first
--   grow, write_bytes(0) over the realloc tail, kernel-zero vm pages.
-- IR pins (p16_*.ll): t's decl ctor emits alm_tbl_new(i64 8, i8 138) —
--   contains_tables (0x80) plus elem_kind table (5) — minted statically
--   even though the loop never stores; the pre-header reserve (b0) and
--   the under-header nested reserve (b1) both target t with the reserve
--   register (latch bound, K_eff folded); the row ctor emits
--   alm_tbl_new(i64 8, i8 0) inside the body and the admitted store rides
--   the inline fast lane (a gep store, no alm_tbl_set_row call, no refuse
--   arm) — but every body site is unreached at runtime because the guard
--   fails on its first test.

local t = {}
for i = 6, 5 do
  t[i] = { i }
end
print(#t)
print(sys_alloc_count())

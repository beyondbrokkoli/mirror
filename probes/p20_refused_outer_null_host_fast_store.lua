-- P20 the refused outer row's null host meets the inner fast store (the
-- 81cb140 + 5cd8af9 + fast-lane-mint composition). Base-1 trip i = 0
-- refuses t[0] (user key 0 -> physical -1): set_core reclaims the row
-- (5cd8af9, RT_FREE fired) and the slot keeps its zero word. The trip's
-- inner fill t[0][j] then loads that zero-lane value as its host
-- (alm_tbl_get answers from_lane(0) for the refused key), drives it
-- through alm_tbl_reserve(null, ...) — RT_RESERVE_NULL fires and reserve
-- returns clean — and lands on the INLINE fast store:
--     %ts8.d = load ptr, ptr %v25, !alias.scope !0
--     %ts8.s = getelementptr inbounds i64, ptr %ts8.d, i64 %v32
--     store i64 %v33, ptr %ts8.s, !noalias !0
-- (libalm_out.ll b5; the plain fast-store mint, src/backend.rs:1557 —
-- the guarded variant at backend.rs:1584 that loads the mode byte was
-- NOT minted). No runtime face runs past the reserve and no
-- null-row-store guard exists (P19's law is read-only), so the process
-- dies by SIGSEGV before its first print — identically in the
-- shared-library face and the exe twin: the fault is baked into the
-- program, not the .so host.
-- Run: lua run.lua probe probes/p20_refused_outer_null_host_fast_store.lua
-- (probe mode never forwards the FLAGS: header; it shows the clean
-- base-0 face. The crash needs the base-1 mints:)
--   target/release/alm --index-base=1 -o /tmp/p20q2.so probes/p20_refused_outer_null_host_fast_store.lua && /tmp/p20q2.so
--   target/release/alm --index-base=1 --for-limit=inclusive -o /tmp/p20q4.so probes/p20_refused_outer_null_host_fast_store.lua && /tmp/p20q4.so
-- Quadrants (2026-11-09 receipts):
--   exclusive base-0: exit 0, prints 3 3 11 0 4 — clean (the row ctor
--     reserve borrows the enclosing latch bound == 3, so #t[1] == 3)
--   base-1 exclusive: SIGSEGV 139, no stdout
--   base-0 inclusive: exit 0, prints 4 4 11 32 5 — clean (bound == 4)
--   base-1 inclusive: SIGSEGV 139, no stdout (the studied quadrant)
-- gdb (base-1 inclusive): #0 alm_exec () at
--   for_inclusive_base1_ctor_reserve_reclaim.lua:15 (t[i][j] = i * 10 + j)
--   => mov 0x0(%r13),%rax — %r13 == the null host; the fault is the
--   header load of the fast mint, one instruction before the gep+store.
-- RT sidecar at death (both faces): 50:RT_ALLOC 60:RT_FREE
--   157:RT_RESERVE_NULL — reclaim and null-reserve both behaved; the
--   inline store is the only unguarded face in between.
-- Fix direction (lowerer, not rt.rs): the fast-store eligibility fact
-- (c0156b0) must decline — or mint the guarded variant — when the host
-- may alias a refused store's slot. The clean base-0 faces show the
-- reserves are not the issue; the host load is.
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

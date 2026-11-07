-- P9 base 1: the --index-base=1 spelling of the same fill. i runs 1..8
-- (exclusive limit 9), stores t[i] = i, so under base 1 the physical
-- cells 0..7 hold 1..8 -- identical memory to p9_canonical and
-- p9_shifted, and identical user-visible output (8, then 1, 8).
-- Run: the compiler directly with the surface flag (the probe runner
-- has no per-file flag slot):
--   target/release/alm --index-base=1 -o target/probe/p9_base1.so \
--     probes/p9_base1.lua && ./target/probe/p9_base1.so
-- Verdicts: the key i is KeyShape::Guard whose physical offset is
-- -index_base = -1; the gate re-proves (entry_const 1, 1 - 1 = 0 >= 0)
-- and the store rides the inline fast lane; the seam lowers the bare
-- guard key with exactly one sub per store (LoadInt 1 + Sub), the
-- tight reserve covers bound + K_eff = 9 - 1 = 8 cells, and #t == 8.
-- Golden: the normalized IR must be IDENTICAL to the tightened
-- p9_shifted (same one-sub-per-store, same reserve constant 8):
--   python3 probes/ir_delta.py target/probe/p9_shifted.ll \
--     target/probe/p9_base1.ll   (exit 0, zero delta)

local t = {}
for i = 1, 9 do
  t[i] = i
end
print(#t)
print(t[1], t[8])

-- FLAGS: --index-base=1
-- EXPECT: 5
-- XFAIL: judge_divergence_audit/zero-key — the store below the base
-- translates to physical -1 and the runtime's negative-index no-op
-- silently drops it; the judge stores user key 0 as a plain hash key
-- and reads it back. Flip when below-base keys stop mapping onto
-- negative physical slots (kept, or rejected loudly).
-- User key 0 is below the base: the store translates to physical -1
-- and the runtime's negative-index no-op drops it; the read
-- translates the same way and answers 0.
local t = {}
t[0] = 5
print(t[0])

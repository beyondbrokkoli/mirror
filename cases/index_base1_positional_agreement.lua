-- FLAGS: --index-base=1
-- EXPECT: 7	8	9
-- EXPECT: 3
-- EXPECT: nil
-- XFAIL: judge_divergence_audit/absent-nil — the below-base read
-- answers 0 (the absent-read zero-fill); the judge answers nil. Flip
-- with the nil-as-value lane (read_before_deciding_store, int_tables).
-- Positional entries and literal keys agree in user space: {7, 8,
-- [3] = 9} reads 7, 8, 9 at user keys 1..3, #t is the border 3, and
-- user key 0 sits below the base (out of the array domain, reads 0).
local t = {7, 8, [3] = 9}
print(t[1], t[2], t[3])
print(#t)
print(t[0])

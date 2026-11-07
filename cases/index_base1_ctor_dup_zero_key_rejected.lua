-- FLAGS: --index-base=1
-- EXPECT_BUILD_FAIL: Syntax Error: duplicate index 0 in constructor
-- The dedup set holds physical slots in every base, negatives
-- included: both literal [0] keys shift to physical -1 at the mint,
-- so the collision fires below the base, and the message renders the
-- user key 0 -- exactly where {[-1] = 1, [-1] = 2} would collide at
-- base 0 minus the double-negation spelling (which base 0 cannot
-- write with a bare literal at all).
local t = { [0] = 1, [0] = 2 }

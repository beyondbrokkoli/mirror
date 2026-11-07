-- FLAGS: --index-base=1
-- EXPECT_BUILD_FAIL: Syntax Error: duplicate index 1 in constructor
-- The dedup set holds physical slots in every base: the positional 10
-- sits at physical 0 and the literal [1] shifts to physical 0 at the
-- mint, so the collision fires exactly where {10, [0] = 5} rejects at
-- base 0. The message renders the user key.
local t = {10, [1] = 5}

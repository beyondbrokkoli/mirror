-- The empty string is concat's identity: an empty operand contributes
-- nothing, and the pool's content dedup hands back the same handle.
-- EXPECT: alpha
-- EXPECT: beta
-- EXPECT: 0
local x = "alpha"
print("" .. x)
local y = "beta"
print(y .. "")
print(#("" .. ""))

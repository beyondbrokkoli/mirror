-- A chain folds its literal pairs before minting runtime concats, so
-- this is two alm_str_concat calls, not five.
-- EXPECT: alpha_1_beta!
-- EXPECT: one-two-three-four
local n = "1"
print("alpha" .. "_" .. n .. "_" .. "beta" .. "!")
print("one" .. "-" .. "two" .. "-" .. "three" .. "-" .. "four")

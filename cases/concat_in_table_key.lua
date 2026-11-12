-- A concatenated key must land where the literal key lands: the
-- product interns to the same pool handle, so the str-key lane cannot
-- tell the minted key from the written one.
-- EXPECT: 7
-- EXPECT: 9
local t = {}
t["a" .. "b"] = 7
local k = "c" .. "d"
t[k] = 9
print(t["ab"])
print(t["cd"])

-- EXPECT: pool
-- EXPECT: pool
-- EXPECT: 4
-- EXPECT: rodata
-- EXPECT: 6
local t = {1, "pool"}
local s = t[1]
print(s)
t = nil
print(s)
print(#s)
local plain = {"rodata"}
local q = plain[0]
plain = nil
print(q)
print(#q)

-- EXPECT: 0	1
-- EXPECT: 1	0

local m = {{}, {1}}
print(m[0][0], m[1][0])

local n = {{1}, {}}
print(n[0][0], n[1][0])

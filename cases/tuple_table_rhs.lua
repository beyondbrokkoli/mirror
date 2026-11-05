-- EXPECT: 7	8
local t = {7}
local u = {8}
local a = {{9}, {9}}
a[0], a[1] = t, u
print(a[0][0], a[1][0])

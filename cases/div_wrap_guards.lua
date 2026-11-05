-- EXPECT: -9223372036854775808	0	9223372036854776000
-- EXPECT: 3	1	3.5	-4	2
-- EXPECT: inf	-inf	NaN

local min = -9223372036854775807 - 1
local mone = 0 - 1
print(min // mone, min % mone, min / mone)

print(7 // 2, 7 % 3, 7 / 2, -7 // 2, -7 % 3)

print(1 / 0, -1 / 0, 0 / 0)

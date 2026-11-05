-- EXPECT: 1.5
-- EXPECT: 2.5	1.5
-- EXPECT: 7
local t = {}
local u = t
u[0] = 1.5
print(u[0])

local pick = {}
if 1 > 2 then
    pick = {}
else
    pick = u
end
pick[1] = 2.5
print(pick[1], pick[0])

local ints = {}
local i = 0
while i < 8 do
    ints[i] = i
    i = i + 1
end
print(ints[7])

-- EXPECT: 41	42
-- EXPECT: 0	7
-- EXPECT: 9900
-- EXPECT: 99	0
-- EXPECT: 5
-- EXPECT: 3

local t = {}
t[0] = 41
t[1] = t[0] + 1
print(t[0], t[1])

t[10] = 7
print(t[5], t[10])

local n = 100
local i = 0
while i < n do
    t[i] = i * 2
    i = i + 1
end
local sum = 0
i = 0
while i < n do
    sum = sum + t[i]
    i = i + 1
end
print(sum)

local u = t
u[20] = 99
print(u[20], u[0])

u = nil
local w = {}
w[0] = 5
print(w[0])

local pick = {}
if 1 > 2 then
    pick = {}
else
    pick = w
end
pick[1] = 3
print(pick[1])

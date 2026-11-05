-- EXPECT: 8
-- EXPECT: 1
-- EXPECT: 0
do
local m = {{7, 8}}
local x = m[0]
print(x[1])
end
do
local t = {}
t[0] = {}
local x = t[0]
x[0] = 1
print(x[0])
end
do
local m = {{7, 8}}
local x = m[5]
print(x[0])
end

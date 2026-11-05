-- EXPECT: 1	2	3	4
-- EXPECT: string
-- EXPECT: string
-- EXPECT: 0
-- EXPECT: 5
do
local m = { {1, 2}, {3, 4} }
print(m[0][0], m[0][1], m[1][0], m[1][1])

local nested = {}
local t = {}
t[0] = nested
t[0][0] = "string"
print(t[0][0])

local u = { {"string"} }
print(u[0][0])

m[0][0] = 9
print(m[0][0] - 9)

local named = {}
do
  local row = {5, 6}
  named[0] = row
  print(named[0][0])
end
end

-- EXPECT: 0
-- EXPECT: 11
-- EXPECT: 3
local t = {}
local u = {3}
local k = u[0] * 100000 + 7
t[k] = 11
local i = 0
while i < 4 do
  t[i] = i
  i = i + 1
end
print(t[0])
print(t[k])
print(t[3])
t = nil
u = nil

-- EXPECT: 0
-- EXPECT: 30
-- EXPECT: 11
local t = {}
local i = 0
while i < 4 do
  t[i] = i
  i = i + 1
end
local u = {3}
local k = u[0] * 100000 + 7
t[k] = 11
local j = 0
while j < 4 do
  t[j] = j * 10
  j = j + 1
end
print(t[0])
print(t[3])
print(t[k])
t = nil
u = nil

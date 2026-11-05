-- EXPECT: 2
-- EXPECT: 1
local t = {}
t[99999+1] = 1
local i = 0
while i < 3 do
  t[i] = i
  i = i + 1
end
print(t[2])
print(t[100000])
t = nil

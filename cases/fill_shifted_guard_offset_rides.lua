-- EXPECT: 3
-- EXPECT: 7
local t = {}
local j = 1
while j < 9 do
  t[j-1] = j - 1
  j = j + 1
end
print(t[3])
print(t[7])

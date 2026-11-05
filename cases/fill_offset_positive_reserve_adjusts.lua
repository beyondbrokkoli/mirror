-- EXPECT: 0	6
-- EXPECT: 3
local t = {}
local i = 0
while i < 3 do
  t[i+1] = i * 3
  i = i + 1
end
print(t[1], t[3])
print(t[2])

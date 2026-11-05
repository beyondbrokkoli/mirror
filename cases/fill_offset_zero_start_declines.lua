-- EXPECT: 5	6
local t = {}
local i = 0
while i < 3 do
  t[i-1] = i + 4
  i = i + 1
end
print(t[0], t[1])

-- EXPECT: 3	4
-- EXPECT: 4	5
local t = {}
local i = 0
while i < 2 do
  t[i] = {}
  t[i][0] = i + 3
  t[i][1] = i + 4
  i = i + 1
end
print(t[0][0], t[0][1])
print(t[1][0], t[1][1])
t = nil

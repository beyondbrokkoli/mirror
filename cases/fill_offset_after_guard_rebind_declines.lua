-- EXPECT: 5	5
-- EXPECT: 8
local t = {}
local i = 0
while i < 4 do
  t[i+1] = i + 4
  if i == 1 then
    i = i + 2
  end
  t[i+1] = i + 5
  i = i + 1
end
print(t[1], t[2])
print(t[4])

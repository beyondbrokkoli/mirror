-- EXPECT: 1
local t = {5, 0, 0, 0, 0, 0}
local i = 0
while i < t[0] do
  t[i] = i
  t[0] = 1
  i = i + 1
end
print(i)
t = nil

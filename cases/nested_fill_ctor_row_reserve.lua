-- EXPECT: 1
local t = {{}}
local i = 0
while i < 2 do
  t[0][i] = i
  i = i + 1
end
print(t[0][1])
t = nil

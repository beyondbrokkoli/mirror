-- EXPECT: 1
-- EXPECT: 2
local t = {[100000+1] = 1}
print(t[100001])
local i = 0
while i < 3 do
  t[i] = i
  i = i + 1
end
print(t[2])
t = nil

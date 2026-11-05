-- EXPECT: 5
local t
local c = 1 < 2
if c then
  t = {[100001] = 1}
else
  t = {7, 8, 9}
end
local i = 0
while i < 3 do
  t[i] = 5
  i = i + 1
end
print(t[0])
t = nil

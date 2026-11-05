-- EXPECT: 5
local t = {[100001] = 1}
local c = 2 < 1
local i = 0
while c do
  t = {7, 8, 9}
  i = i + 1
end
local j = 0
while j < 3 do
  t[j] = 5
  j = j + 1
end
print(t[0])
t = nil

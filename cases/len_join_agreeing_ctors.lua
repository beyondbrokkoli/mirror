-- EXPECT: 3
local t
local c = 1 < 2
if c then
  t = {1, 2, 3}
else
  t = {4, 5, 6}
end
print(#t)

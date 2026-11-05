-- EXPECT: 0
local t = {}
local x = t[0]
if 1 < 2 then
  t[0] = 5
end
print(x)
t = nil

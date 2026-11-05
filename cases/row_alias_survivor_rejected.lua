-- EXPECT: 7

local x = {}
do
  local t = {{7, 8}}
  x = t[0]
end
print(x[0])

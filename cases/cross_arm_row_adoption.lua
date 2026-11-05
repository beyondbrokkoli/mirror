-- EXPECT: 0
local t = {{1}}
local u = {{}}
local r
if 2 < 1 then
  r = t[0]
else
  r = u[0]
end
print(r[0])

-- EXPECT: 16	1
-- EXPECT: 16	1
local out = {x = 0, y = 0, z = 0, w = 0}
local mm = {13, 1, 2, 1}
local x, y, z, w = 1, 1, 1, 1
out.x, out.y = mm[0]*x + mm[1]*y + mm[2]*z, mm[3]*w
print(out.x, out.y)
out.z, out.w = out.x, out.y
print(out.z, out.w)

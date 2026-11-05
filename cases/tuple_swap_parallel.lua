-- EXPECT: 2	1
-- EXPECT: 8	7
local o = {1, 2}
o[0], o[1] = o[1], o[0]
print(o[0], o[1])
local p = {7, 8}
p[0], p[1] = p[1], p[0] + 0
print(p[0], p[1])

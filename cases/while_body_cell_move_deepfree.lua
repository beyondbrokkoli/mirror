-- EXPECT: 5	6
-- EXPECT: 0

local outer = {}
local i = 0
while i < 2 do
    local t = {}
    t[0] = i + 5
    outer[i] = t
    i = i + 1
end
print(outer[0][0], outer[1][0])
outer = nil
print(sys_alloc_count())

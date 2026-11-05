-- EXPECT: 22
-- EXPECT: 1
-- EXPECT: 0

local keep = {}
keep[0] = -1
local i = 0
while i < 3 do
    local t = {}
    t[0] = i * 11
    keep = t
    i = i + 1
end
print(keep[0])
print(sys_alloc_count())
keep = nil
print(sys_alloc_count())

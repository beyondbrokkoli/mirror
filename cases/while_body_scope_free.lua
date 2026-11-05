-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 0

local i = 0
while i < 2 do
    local t = {}
    t[0] = i
    print(t[0])
    i = i + 1
end
print(sys_alloc_count())

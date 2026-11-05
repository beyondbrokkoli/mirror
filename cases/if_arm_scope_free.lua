-- EXPECT: 1
-- EXPECT: 0

if 1 < 2 then
    local t = {}
    t[0] = 1
    print(t[0])
end
print(sys_alloc_count())

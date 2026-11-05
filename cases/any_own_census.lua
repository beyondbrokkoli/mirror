-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 1
-- EXPECT: 100001
-- EXPECT: 2
-- EXPECT: 1
-- EXPECT: 0
print(sys_alloc_count())
local t = {1, 2.5, true, "x"}
print(sys_alloc_count())
t[100000] = 7
print(sys_alloc_count())
print(#t)
do
    local u = {t[0], t[3]}
    print(sys_alloc_count())
end
print(sys_alloc_count())
t = nil
print(sys_alloc_count())

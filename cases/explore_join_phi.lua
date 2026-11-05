-- EXPECT: 7
-- EXPECT: 0
do
    local a = {}
    local b = {}
    local pick
    if 1 < 2 then
        pick = a
    else
        pick = b
    end
    pick[0] = 7
    print(pick[0])
    pick = nil
end
print(sys_alloc_count())

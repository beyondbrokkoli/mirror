-- EXPECT: 9
-- EXPECT: 1
-- EXPECT: true
-- EXPECT: true
-- EXPECT: 0
if ({1})[0] == 1 then print(9) end
local i = 0
while ({5})[i] == 5 do i = i + 1 end
print(i)
print(({1})[0] == 1 and ({2})[0] == 2)
print(({1})[0] == 9 or ({2})[0] == 2)
print(sys_alloc_count())

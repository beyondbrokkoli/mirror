-- ARGS: 4
-- EXPECT: 4
-- EXPECT: 2
local i = 0
while i < arg[0] do
    i = i + 1
end
print(i)
local t = {1, 2}
print(sys_alloc_count())

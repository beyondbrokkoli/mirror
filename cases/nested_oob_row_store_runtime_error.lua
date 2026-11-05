-- EXPECT_PANIC: alm runtime error: attempt to index a nil value
local m = {{7, 8}}
m[5][0] = 9
print(m[5][0])

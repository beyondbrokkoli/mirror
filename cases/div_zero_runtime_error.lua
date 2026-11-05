-- EXPECT_PANIC: alm runtime error: attempt to perform 'n/0' (integer division by zero)

local z = 0
print(5 // z)

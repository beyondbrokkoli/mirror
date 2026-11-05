-- ARGS: 0
-- EXPECT_PANIC: alm runtime error: attempt to perform 'n/0'
print(10 % arg[0])

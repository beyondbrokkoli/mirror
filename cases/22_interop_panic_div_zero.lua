-- EXPECT: before the divide
-- EXPECT_PANIC: alm runtime error: attempt to perform 'n/0' (integer division by zero)
print("before the divide")
print(1 // 0)

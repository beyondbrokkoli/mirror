-- EXPECT_BUILD_FAIL: '#' requires a Table, String, or Any operand — 'x' is a scalar
do
    local x = 5
    print(#x)
end

-- EXPECT_BUILD_FAIL: '#' requires a Table, String, or Any operand — 'f' is a scalar
do
    local f = 1.5
    print(#f)
end

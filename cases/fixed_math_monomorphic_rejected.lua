-- EXPECT_BUILD_FAIL: calls are monomorphic
local Fixed = {}
function Fixed.mul(a, b)
    return (a * b) // 1024
end
print(Fixed.mul(1024, 2048))
print(Fixed.mul(1.5, 2.5))

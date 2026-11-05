-- EXPECT_BUILD_FAIL: Type Error: 'a' is recursive
local function a(n)
    if n > 0 then
        return b(n - 1)
    end
    return 0
end
local function b(n)
    if n > 0 then
        return a(n - 1)
    end
    return 0
end
print(a(3))

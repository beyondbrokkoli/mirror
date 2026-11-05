
-- EXPECT_BUILD_FAIL: line 8:
-- EXPECT_BUILD_FAIL: 't' is moved more than once
-- EXPECT_BUILD_FAIL: line 16:

do
    local t = {1, 2}
    local a, b = t, t
    a[0] = 1
    print(b[0])
end

do
    local t = {1, 2}
    local a = t
    t[0] = 1
    print(a[0])
end

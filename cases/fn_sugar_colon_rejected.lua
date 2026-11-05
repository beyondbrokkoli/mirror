-- EXPECT_BUILD_FAIL: colon methods are not surface
function M:area()
    return 1
end

-- EXPECT_BUILD_FAIL: Type Error: 'break' cannot leave an expansion
while true do
    local f = function() break end
end

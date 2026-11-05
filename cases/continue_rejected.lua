-- EXPECT_BUILD_FAIL: Syntax Error: continue is not in the dialect
while true do
    local n = 1
    continue
end

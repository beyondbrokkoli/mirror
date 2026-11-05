-- EXPECT_BUILD_FAIL: Syntax Error: cannot assign to for-loop variable 'i'
for i = 0, 4 do
    local n = 0
    while n < 3 do
        i = i + 1
        n = n + 1
    end
end
print("unreached")

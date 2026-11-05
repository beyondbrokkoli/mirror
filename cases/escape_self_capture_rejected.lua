-- EXPECT_BUILD_FAIL: Scope Error: reference to undeclared variable 'B'
local function Make()
    local function B(x)
        if x > 0 then
            return { me = B }
        end
        return { me = nil }
    end
    return B(1)
end
print(Make())

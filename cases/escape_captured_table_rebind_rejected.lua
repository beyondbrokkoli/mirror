-- EXPECT_BUILD_FAIL: line 4: Type Error: 'u' is a captured table
local function Make()
    local u = { [0] = 1 }
    local function Step() u = { [0] = 2 } end
    return Step
end
local s = Make()
s()
print("unreached")

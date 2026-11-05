-- EXPECT_BUILD_FAIL: Type Error: 'Base' cannot be created inside a conditional arm
local function Base() return 1 end
local t = { [0] = 1 }
if t[0] == 1 then
    local f = Base
    print(f())
end
print("unreached")

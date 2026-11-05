-- EXPECT_BUILD_FAIL: Type Error: 't' is not a module surface
local function Make()
    local function Get() return 7 end
    return { get = Get }
end
do
    local t = Make()
    print(t.get())
end

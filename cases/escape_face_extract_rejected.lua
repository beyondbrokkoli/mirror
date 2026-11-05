-- EXPECT_BUILD_FAIL: Type Error: the face rides the field
local function Make()
    local n = 41
    local function Get() n = n + 1 return n end
    return { get = Get }
end
local t = Make()
local g = t.get
print("alive")
print(g())

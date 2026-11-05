-- EXPECT: 42
-- EXPECT: 43
local function Make()
    local n = 41
    local function Get() n = n + 1 return n end
    return { get = Get }
end
local t = Make()
local r = { get = t.get }
print(r.get())
print(t.get())

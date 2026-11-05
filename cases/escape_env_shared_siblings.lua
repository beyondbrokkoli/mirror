-- EXPECT: 42
local function Make()
    local buf = { 0 }
    local function Put(v) buf[0] = v end
    local function Get() return buf[0] end
    return { Put = Put, Get = Get }
end
local d = Make()
d.Put(41)
d.Put(d.Get() + 1)
print(d.Get())

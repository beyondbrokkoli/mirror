-- EXPECT: 15
local function Make(scale)
    local base = 7
    local function Poll(x) return x + scale + base end
    return { Poll = Poll }
end
local d = Make(3)
print(d.Poll(5))

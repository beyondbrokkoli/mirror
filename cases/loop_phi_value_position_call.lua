-- EXPECT: 10
local t = { [0] = 7 }
local acc = 0
local function H()
    acc = acc + 1
    return t[0]
end
local function G()
    return H()
end
local function F()
    local q
    local a = G()
    local n = 0
    while n < 9 do
        n = n + 1
        local c = G()
    end
    return 0
end
F()
print(acc)

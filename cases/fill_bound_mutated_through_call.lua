-- EXPECT: 1
local t = {}
local b = 3
local n = 0
local function Shrink()
    b = 1
    return 0
end
while n < b do
    t[n] = n * 2
    n = n + 1
    Shrink()
end
print(#t)

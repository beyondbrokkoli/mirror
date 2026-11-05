-- EXPECT_BUILD_FAIL: Lifetime Error: 'x' may be nil here — table reads and stores through a possibly-nil name are rejected at compile time

local x
local i = 0
while i < 2 do
    local t = {{7, 8}}
    x = t[0]
    i = i + 1
end
print(x[1])

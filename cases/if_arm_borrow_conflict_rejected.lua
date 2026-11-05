-- EXPECT_BUILD_FAIL: Lifetime Error: 'x' may be nil here — table reads and stores through a possibly-nil name are rejected at compile time

local x
if 1 < 2 then
    local t = {{7, 8}}
    x = t[0]
end
print(x[1])


-- EXPECT_BUILD_FAIL: Lifetime Error: 't' may be nil here — table reads and stores through a possibly-nil name are rejected at compile time
do
    local t = {}
    t[0] = 7
    t = nil
    local x = t[0]
    print(x)
end

-- EXPECT_BUILD_FAIL: a table value never rides a cell
do
    local nc = {}
    nc[0] = nil
end

-- EXPECT_BUILD_FAIL: Lifetime Error: 't' may be nil here — table reads and stores through a possibly-nil name are rejected at compile time
do
    local t = {}
    t[0] = 1
    if 2 > 3 then
        t = nil
    end
    print(t[0])
end

-- EXPECT_BUILD_FAIL: a table is read before it is ever given a value
do
    local u = {}
    print(u[0])
end

-- EXPECT_BUILD_FAIL: table index overflow
do
    local w = {}
    local k = 0
    while k < 1000 do
        w[k] = k
        k = k + 1
    end
    w[9223372036854775807] = 42
end

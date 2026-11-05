
-- EXPECT_BUILD_FAIL: '+' requires numeric operands
do
    local t = {}
    local x = t + 1
end

-- EXPECT_BUILD_FAIL: cannot print a table
do
    local t = {}
    print(t)
end


-- EXPECT_BUILD_FAIL: 'nil' releases tables — 'x' is of type Integer
do
    local x = 5
    x = nil
end

-- EXPECT_BUILD_FAIL: 'nil' is only valid as the right-hand side of 't = nil'
do
    local nl = nil
end

-- EXPECT_BUILD_FAIL: 'while' condition must be a Boolean, got IntTable
do
    local tc = {}
    while tc do
        tc[0] = 1
    end
end

-- EXPECT_BUILD_FAIL: table index must be an Integer or a String, got Float
do
    local tg = {}
    local xg = tg[2.5]
end

-- EXPECT_BUILD_FAIL: table index must be an Integer or a String, got Float
do
    local tf = {}
    tf[1.5] = 1
end

-- EXPECT_BUILD_FAIL: cannot index Integer
do
    local xi = 5
    local yi = xi[0]
end

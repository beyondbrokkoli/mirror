-- EXPECT: 1
-- EXPECT: 2
-- EXPECT: 2
local function Make(start)
    local count = start
    local function Bump()
        count = count + 1
        return count
    end
    local function Peek() return count end
    return { Bump = Bump, Peek = Peek }
end
local d = Make(0)
print(d.Bump())
print(d.Bump())
print(d.Peek())

-- EXPECT: 3
-- EXPECT: 2
local function Make(scale)
    local buf = { 0, 0 }
    local function Poll(i, v)
        buf[i] = v
        return buf[i]
    end
    return { Poll = Poll }
end
local d = Make(5)
print(d.Poll(0, 3))
d = nil
print(sys_alloc_count())

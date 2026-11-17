-- EXPECT: 3
-- EXPECT: 3
-- The count pin rode 2 before the p9 reversal (2026-11-18): boxing
-- the captured `buf` adds one live allocation at the count print —
-- box + buf table + env — because the box's drain rides the root
-- exit, after this print.
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

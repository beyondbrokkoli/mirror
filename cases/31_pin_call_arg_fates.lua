-- EXPECT: 0
-- EXPECT: 7
-- EXPECT: 5
-- EXPECT: 4
-- EXPECT: 4
-- The last two count pins rode 3 before the p9 reversal (2026-11-18):
-- boxing the captured `carrier` adds one live allocation at each
-- count print — the box cell, drained at the root exit after the
-- prints. carrier(2) + box + mv's env.
local st = function(t) t[9] = 1 end
st({})
print(sys_alloc_count())
local carrier = {{9}}
local mv = function(x) carrier[1] = x end
mv({7, 8})
print(carrier[1][0])
local br = function(x) return x end
local held = br({5})
print(held[0])
held = nil
print(sys_alloc_count())
br({6})
print(sys_alloc_count())

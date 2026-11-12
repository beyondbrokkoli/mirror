-- assert's fail lane: a false condition raises the interned message
-- through the runtime trap — the M2 assert law's one death.
-- EXPECT_PANIC: alm runtime error: assertion failed! size must be positive

local ok = 1 > 2
assert(ok, "size must be positive")
print("unreachable")

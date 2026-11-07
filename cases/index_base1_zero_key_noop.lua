-- FLAGS: --index-base=1
-- EXPECT: 0
-- User key 0 is below the base: the store translates to physical -1
-- and the runtime's negative-index no-op drops it; the read
-- translates the same way and answers 0.
local t = {}
t[0] = 5
print(t[0])

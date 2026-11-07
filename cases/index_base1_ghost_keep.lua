-- FLAGS: --index-base=1
-- EXPECT: 21
-- EXPECT: 22
-- EXPECT: 2
-- A row read at a literal key stays housable under base 1: the ghost
-- keep records the physical slot (1 -> 0) and the later reads through
-- the housing resolve. The alloc count pins that the housed row frees
-- with its housing's drop.
local a = {{21, 22}, {23}}
local t = {}
t[1] = a[1]
print(t[1][1])
print(t[1][2])
t = nil
print(sys_alloc_count())

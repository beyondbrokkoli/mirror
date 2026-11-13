-- EXPECT: 2	3
-- EXPECT: 2
-- The double-buffer idiom (T2): 'cur, nxt = nxt, cur' in a loop body.
-- Each table moves once into its value temp and back out through a
-- rebind, every trip — the swap is two moves, never a copy, so the
-- two ctors stay the only allocations. The judge prints 2 3.
local cur = {0, 0}
local nxt = {1, 1}
local i = 0
while i < 3 do
  cur[0] = cur[0] + nxt[0]
  cur, nxt = nxt, cur
  i = i + 1
end
print(cur[0], nxt[0])
print(sys_alloc_count())

-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 8
-- EXPECT: 0
-- EXPECT: 1
-- EXPECT: 4
-- EXPECT: 9
-- The single-pass battery pin of P19 Q4 (sparse stride keys under base-1
-- inclusive): i * 2 declines the fill-loop conversion, every row store
-- rides the generic face, and the border tracks the span high-water at
-- 8 (physical 1, 3, 5, 7; gap cells count in the border and answer the
-- zero lane). The read pair pins the base-1 spelling law at two depths:
-- r[0][0] reads the refused null row (zero lane -> 0) while r[1][1] is
-- the grandchild scalar, and t[8][1][1] pins the far row's nested
-- housing. Single pass: the set_core replace-drop law (P21) is stroked
-- by nothing, so the ledger closes at 9 with a clean zero-leak exit.
local t = {}
for i = 1, 4 do
  t[i * 2] = { { i } }
end
print(#t)
local r = t[2]
print(r[0][0])
print(r[1][1])
print(t[8][1][1])
print(sys_alloc_count())

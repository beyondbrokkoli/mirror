-- FLAGS: --index-base=1 --for-limit=inclusive
-- EXPECT: 5
-- EXPECT: 1
-- EXPECT: 40
-- EXPECT: 5
-- Exercises 81cb140 (row ctor reserve before the store face) under base-1
-- inclusive limits (the battery pin of P18 Q4): trip i = 0 sizes the row
-- from the enclosing loop's pre-header bound before set_core refuses
-- physical key -1 and reclaims + poisons it; trips 1..4 house at 0..3.
local t = {}
for i = 0, 4 do
  t[i] = { i, i * 10 }
end
print(#t)
print(t[1][1])
print(t[4][2])
print(sys_alloc_count())

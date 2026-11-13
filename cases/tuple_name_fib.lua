-- EXPECT: 55	89
-- The customer shape itself (T2): the Fibonacci step
-- 'a, b = b, a + b' — both values latch against the old names before
-- either rebinds, every trip. Ten trips under the exclusive for-limit;
-- the judge prints 55 89 over the same ten (its 'for i = 0, 9').
local a, b = 0, 1
for i = 0, 10 do
  a, b = b, a + b
end
print(a, b)

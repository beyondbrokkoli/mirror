-- FLAGS: --index-base=1
-- EXPECT: 8
-- EXPECT: 1	0
-- A zero-start fill under base 1 has a negative first physical key
-- (0 - 1 = -1), so the admission gate declines the fast lane and the
-- store rides the checked arm, where the runtime's negative-index
-- no-op law drops the first write. The declined store contributes no
-- offset, so the reserve keeps the raw bound (the base-0 declined-fill
-- behavior, translated): the border over-reports by exactly the
-- out-of-domain write.
local t = {}
for i = 0, 8 do
  t[i] = i
end
print(#t)
print(t[1], t[8])

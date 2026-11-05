-- EXPECT: 2
-- EXPECT: 2
-- EXPECT: 1	2
do
  local i = 0
  local t = {[i] = 1, [i] = 2}
  print(t[i])
  local m = {[0] = 1, [i] = 2}
  print(m[0])
  local j = 5
  local n = {[0] = 1, [j] = 2}
  print(n[0], n[j])
end

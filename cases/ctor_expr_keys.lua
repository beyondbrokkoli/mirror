-- EXPECT: 7
-- EXPECT: 40	0
-- EXPECT: 9	6
-- EXPECT: 0	0
-- EXPECT: 3
do
  local k = 1
  local t = {[0 + 1] = 7}
  print(t[1])
  local u = {[k * 2] = 40}
  print(u[2], u[0])
  local pos = {5, [k - 1] = 9, 6}
  print(pos[0], pos[1])
  local neg = {[k - 2] = 9}
  print(neg[-1], neg[0])
  print(#{1, 2, 3})
end

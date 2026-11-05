-- EXPECT: 2
-- EXPECT: 2
do
  print(#{[0 + 1] = 7})
end
do
  local k = 1
  local t = {1, [k] = 2}
  print(#t)
end

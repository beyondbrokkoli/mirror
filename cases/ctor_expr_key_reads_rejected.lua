-- EXPECT_BUILD_FAIL: Lifetime Error: 'kk' may be nil here — reads through a possibly-nil name are rejected at compile time

do
  local kk
  local t = {[kk] = 1}
end
do
  local n = {3}
  local q = n
  local t = {[n[0]] = 1}
end
do
  local outer = {{1}}
  local k = 0
  local t = {[k] = outer[0]}
end
do
  local inner = {1}
  local k = 0
  local t = {[k] = inner}
  print(inner[0])
end

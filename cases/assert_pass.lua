-- assert's pass lane: the guard runs and the condition's value rides
-- home (assert answers its condition, the Lua law).
-- EXPECT: ok
assert(true)
assert(1 == 1, "cannot happen")
if assert(2 > 1) then
  print("ok")
end

-- EXPECT_BUILD_FAIL: cannot assign to for-loop variable 'i'
-- Sugar inside sugar (T2): the for-counter law reads the tuple
-- expansion's name store like any plain '=' — a name target may not
-- rebind the loop variable, whatever spelling rebinds it.
local x = 0
for i = 0, 3 do
  i, x = x, i
end

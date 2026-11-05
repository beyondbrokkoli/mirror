-- EXPECT_BUILD_FAIL: Type Error: table index must be an Integer or a String, got Float
-- EXPECT_BUILD_FAIL: Type Error: table index must be an Integer or a String, got Boolean
do
  local t = {[1.5] = 1}
end
do
  local t = {[true] = 1}
end

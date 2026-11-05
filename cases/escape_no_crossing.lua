-- EXPECT_BUILD_FAIL: Type Error: a closure cannot cross a module boundary
local function Tick(x) return x + 1 end
local pkt = { seed = 1, step = Tick }
return pkt

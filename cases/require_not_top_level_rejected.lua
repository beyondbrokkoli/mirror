-- top-level-only + a named rejection: a require inside a function
-- body has no chunk of its own.
-- FLAGS: --module-root=cases/modules
-- EXPECT_BUILD_FAIL: require is top-level-only
local function late()
    local G = require("child.greeter")
    return G.hello("x")
end
print(late())

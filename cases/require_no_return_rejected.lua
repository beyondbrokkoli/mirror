-- a module's top-level 'return <table>' is the binding — a chunk
-- without one is a named rejection.
-- FLAGS: --module-root=cases/modules
-- EXPECT_BUILD_FAIL: returns nothing
local T = require("no_return.empty")
print("unreachable")

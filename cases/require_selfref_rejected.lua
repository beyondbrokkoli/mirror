-- a module fn referencing its own table cannot ride the record
-- literal — named rejection until the closed-vtable lane exists.
-- FLAGS: --module-root=cases/modules
-- EXPECT_BUILD_FAIL: self-referential module function
local C = require("selfref.counter")

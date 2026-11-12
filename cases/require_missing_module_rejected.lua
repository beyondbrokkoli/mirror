-- one file per dotted name, anchored at --module-root: a name with no
-- file is a named rejection, not a search.
-- FLAGS: --module-root=cases/modules
-- EXPECT_BUILD_FAIL: no module at
local Ghost = require("ghost.nowhere")

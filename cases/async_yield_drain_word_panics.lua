-- EXPECT_PANIC: alm fatal: async.yield of
-- EXPECT_PANIC: the drain word (i64::MIN) is reserved
local async = require("async")
local drain = -9223372036854775807 - 1
async.yield(drain)

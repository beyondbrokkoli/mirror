-- EXPECT_PANIC: alm fatal: coroutine.yield of
-- EXPECT_PANIC: the drain word (i64::MIN) is reserved
local coroutine = require("coroutine")
local drain = -9223372036854775807 - 1
coroutine.yield(drain)

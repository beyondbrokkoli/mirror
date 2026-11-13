-- EXPECT_PANIC: alm fatal: async.resume on handle
local async = require("async")
local t = {}
async.resume(-1, t)

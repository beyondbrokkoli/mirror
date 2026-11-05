-- EXPECT_PANIC: alm fatal: coroutine.resume on handle
local coroutine = require("coroutine")
local t = {}
coroutine.resume(-1, t)

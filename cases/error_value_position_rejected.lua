-- error answers no value: the checker rejects it in value position
-- with a named diagnostic — the raise is a statement, full stop.
-- EXPECT_BUILD_FAIL: builtin 'error' raises

local x = error("no value rides home")

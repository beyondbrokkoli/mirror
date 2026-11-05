-- EXPECT_BUILD_FAIL: Type Error: cyclic table type — a table's element type may not contain the table itself

local t = {}
t = {t}

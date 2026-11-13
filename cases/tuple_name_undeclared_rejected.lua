-- EXPECT_BUILD_FAIL: assignment to undeclared variable 'r'
-- A name target is a rebind, never a declaration (T2): the judge
-- would mint globals here; the dialect has none, so the scope law
-- answers. The stores land right-to-left, so the rightmost undeclared
-- target reports first — and the report carries the header's line.
q, r = 1, 2
print(q, r)

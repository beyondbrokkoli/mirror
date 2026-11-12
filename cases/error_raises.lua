-- error's one verb: the unconditional raise through the same trap as
-- assert's fail path — one machinery, two verbs.
-- EXPECT_PANIC: alm runtime error: uncaught error: bad_allocator

local why = "bad" .. "_" .. "allocator"
error(why)
print("unreachable")

-- EXPECT: 42
-- EXPECT: 42
-- Finding B matrix v4 (the receipts' mutate-only probe): the
-- top-level fn READS AND WRITES root x; the caller's frame only
-- reads x before shadowing it. Pre-fix alm printed 1042, 2 — the
-- write landed on the caller's binding. The capture view redirects
-- the body's writes to the root slot.
-- top-level fn reads AND writes root x, called from a frame that only READS x first
local x = 2
local function F()
    x = x + 40
    return x
end
local function M()
    local y = x
    local x = y + 1000
    return F()
end
print(M())
print(x)

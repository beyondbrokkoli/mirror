-- EXPECT_BUILD_FAIL: Lifetime Error: a row read with a dynamic or unproven key cannot be stored — its birth site is not visible to the lattice; store a row read through a literal key instead

do
local t2 = {}
t2[0] = {}
local row = t2[0]
local t = {}
t[0] = row
print(t[0][0])
end

-- EXPECT_BUILD_FAIL: Lifetime Error: a row read with a dynamic or unproven key cannot be stored — its birth site is not visible to the lattice; store a row read through a literal key instead
do
local t = {}
t[0] = {}
local t3 = {}
t3[0] = t[0]
print(t3[0][0])
end

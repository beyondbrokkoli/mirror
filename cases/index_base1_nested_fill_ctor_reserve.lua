-- FLAGS: --index-base=1
-- EXPECT: 3
-- EXPECT: 2
-- EXPECT: 1	3
-- A P12-style conversion loop under base 1: the parent's pre-header
-- reserve and every fresh row's ctor reserve borrow the WIDENED
-- register (raw bound 4 plus the loop's key_offset -1 = 3), never the
-- condition's raw bound -- the condition keeps testing 4 while all
-- three reserves cover 3 cells, so #outer equals the logical length 3
-- and each row reads back through user keys (outer[2] is the row
-- minted at i = 2, whose first cell user key 1 holds 2).
local outer = {}
for i = 1, 4 do
  outer[i] = {i}
end
print(#outer)
local row = outer[2]
print(row[1])
print(outer[1][1], outer[3][1])

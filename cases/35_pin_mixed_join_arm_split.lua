-- EXPECT: 99
-- EXPECT: 4
-- EXPECT: 9
-- EXPECT: 1
-- ARGS: 1
local mix_base = {{9, 8}, {7}}
local mix
if arg[0] == 1 then mix = {99} else mix = mix_base[0] end
print(mix[0])
mix = nil
print(sys_alloc_count())
print(mix_base[0][0])
mix_base = nil
print(sys_alloc_count())

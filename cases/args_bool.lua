-- ARGS: true
-- EXPECT: 1
-- EXPECT: false
if arg[0] then
    print(1)
end
print(not arg[0])

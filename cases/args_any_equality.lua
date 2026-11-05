-- ARGS: 5 5 hello
-- EXPECT: true
-- EXPECT: false
-- EXPECT: false
-- EXPECT: true
print(arg[0] == arg[1])
print(arg[0] == arg[2])
print(arg[0] ~= arg[1])
print(arg[2] == arg[2])

-- EXPECT: 41
-- EXPECT: rebound
-- ARGS: 41
print(arg[0])
arg = nil
print("rebound")

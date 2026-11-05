-- EXPECT: 1
do
    local t = {}
    t[0] = 0.5
    t = {}
    t[0] = 1
    print(t[0])
end

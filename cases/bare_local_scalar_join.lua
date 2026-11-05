-- EXPECT: 2	1
-- EXPECT: 9	7
-- EXPECT: 1.5
do
  local arr = {}
  arr[0] = 1
  arr[1] = 2
  local temp
  local i = 0
  while i < 2 do
    temp = arr[i]
    arr[i] = arr[i + 1]
    arr[i + 1] = temp
    i = i + 2
  end
  print(arr[0], arr[1])
end
do
  local arr = {}
  arr[0] = 7
  arr[1] = 9
  local temp
  if 1 < 2 then
    temp = arr[0]
    arr[0] = arr[1]
    arr[1] = temp
  end
  print(arr[0], arr[1])
end
do
  local fs = {}
  fs[0] = 1.5
  local pick
  local i = 0
  while i < 1 do
    pick = fs[i]
    fs[1] = pick
    i = i + 1
  end
  print(fs[1])
end

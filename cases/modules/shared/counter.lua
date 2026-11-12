-- memoization witness: the top-level print runs once no matter how
-- many names bind the module; the running total is closure-captured
-- shared state (env-cell write-through, one cell per module)
print("shared: expanding once")
local total = 0
local function add(k) total = total + k return total end
local function get() return total end
local Counter = { add = add, get = get }
return Counter

-- self-referential module fn: bump touches Counter, its own table
local Counter = {}
function Counter.bump() Counter.n = 1 return Counter.n end
return Counter

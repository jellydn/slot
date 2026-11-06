pico-8 cartridge // authored Slot serialization contract fixture
version 42
__lua__
local counter=1
local stats={score=3}
local alias=stats
local co=cocreate(function()
 local n=1
 while true do
  n+=1
  yield(n)
 end
end)
local coroutine_value=0
local function advance()
 counter+=1
 stats.score+=2
 local ok,n=coresume(co)
 assert(ok)
 coroutine_value=n
 if counter==2 then removed=6 end
 if counter>2 then removed=nil future=9 end
end
function _update()
 if btnp(0) then advance() end
end
function _draw()
 cls(0)
 pset(0,0,counter%16)
 pset(1,0,stats.score%16)
 pset(2,0,alias.score%16)
 pset(3,0,removed or 0)
 pset(4,0,future or 0)
 pset(5,0,coroutine_value%16)
end

pico-8 cartridge // authored Slot input, timing, PRNG and noise-audio fixture
version 42
__lua__
local presses=0
local random_colour=0
function _init()
 srand(123)
 for i=0,31 do poke2(0x3200+i*2,0x0f98) end
 poke(0x3241,8)
 poke(0x3242,0)
 poke(0x3243,32)
 sfx(0,0)
end
function _update60()
 if btnp(0) then presses+=1 end
 random_colour=flr(rnd(16))
end
function _draw()
 cls(0)
 pset(0,0,presses%16)
 pset(1,0,flr(t()*60)%16)
 pset(2,0,random_colour)
end

pico-8 cartridge // http://www.pico-8.com
version 42
__lua__
-- Authored test content. No third-party cart, sprites or music.
function _init()
 cartdata('slot_test')
 dset(0,dget(0)+1)
 n=0
 poke(0x3200,33)
 poke(0x3201,14)
 poke(0x3241,8)
 sfx(0)
end
function _update60()
 n+=1
end
function _draw()
 cls(dget(0))
 pset(0,0,n%16)
 if btn(4) then pset(127,127,8) end
end

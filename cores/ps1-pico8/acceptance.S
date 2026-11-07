# Original offline PS1 acceptance program. No SDK, BIOS or game assets.
# Loaded at 0x80010000. Fixed data addresses avoid linker relocations.
.set noreorder
.set noat
.text
    lui $a0, 0x8001
    ori $a0, $a0, 0x4000
    li $a1, 34
    lui $a2, 0x8001
    ori $a2, $a2, 0x4040
    li $a3, 34
    li $t1, 0x12
    jal 0xb0
    nop
    li $t1, 0x13
    jal 0xb0
    nop
    li $a0, 1
    li $t1, 0x4a
    jal 0xb0
    nop
    li $t1, 0x4b
    jal 0xb0
    nop
    li $t1, 0x70
    jal 0xa0
    nop

    # Open existing first. CREATE is not open-or-create on OpenBIOS.
    lui $a0, 0x8001
    ori $a0, $a0, 0x1000
    li $a1, 3
    li $t1, 0x32
    jal 0xb0
    nop
    move $s0, $v0
    bgez $v0, read_card
    nop
    lui $a0, 0x8001
    ori $a0, $a0, 0x1000
    lui $a1, 1
    ori $a1, $a1, 0x203
    li $t1, 0x32
    jal 0xb0
    nop
    bltz $v0, failed
    move $s0, $v0
    # Initialise a new file explicitly; erased card data is unspecified.
    lui $t0, 0x8001
    ori $t0, $t0, 0x3000
    sb $zero, 0($t0)
    b increment
    nop
read_card:
    move $a0, $s0
    lui $a1, 0x8001
    ori $a1, $a1, 0x3000
    li $a2, 128
    li $t1, 0x34
    jal 0xb0
    nop
    li $t0, 128
    bne $v0, $t0, failed
    nop
increment:
    lui $t0, 0x8001
    ori $t0, $t0, 0x3000
    lbu $t2, 0($t0)
    nop
    addiu $t2, $t2, 1
    sb $t2, 0($t0)
    li $t2, 0x53
    sb $t2, 1($t0)
    move $a0, $s0
    move $a1, $zero
    move $a2, $zero
    li $t1, 0x33
    jal 0xb0
    nop
    bnez $v0, failed
    nop
    move $a0, $s0
    lui $a1, 0x8001
    ori $a1, $a1, 0x3000
    li $a2, 128
    li $t1, 0x35
    jal 0xb0
    nop
    li $t0, 128
    bne $v0, $t0, failed
    nop
    move $a0, $s0
    li $t1, 0x36
    jal 0xb0
    nop
    bltz $v0, failed
    nop

    lui $s4, 0x1f80
    lui $t0, 0x0300
    sw $t0, 0x1814($s4)
    lui $t0, 0x0800
    ori $t0, $t0, 1
    sw $t0, 0x1814($s4)
    lui $t0, 0x0500
    sw $t0, 0x1814($s4)
    lui $t0, 0x06c6
    ori $t0, $t0, 0x0260
    sw $t0, 0x1814($s4)
    lui $t0, 0x0704
    ori $t0, $t0, 0x2018
    sw $t0, 0x1814($s4)

    # Own SPU output: key off every firmware voice, disable CD/reverb,
    # then run noise on voice 0 with its volume controlled by RIGHT.
    li $t0, -1
    sh $t0, 0x1d8c($s4)
    sh $t0, 0x1d8e($s4)
    sh $zero, 0x1d84($s4)
    sh $zero, 0x1d86($s4)
    sh $zero, 0x1d98($s4)
    sh $zero, 0x1d9a($s4)
    li $t0, 0x3fff
    sh $t0, 0x1d80($s4)
    sh $t0, 0x1d82($s4)
    sh $zero, 0x1c00($s4)
    sh $zero, 0x1c02($s4)
    li $t0, 0x000f
    sh $t0, 0x1c08($s4)
    sh $zero, 0x1c0a($s4)
    li $t0, 1
    sh $t0, 0x1d94($s4)
    sh $zero, 0x1d96($s4)
    sh $t0, 0x1d88($s4)
    li $t0, 0xdc00
    sh $t0, 0x1daa($s4)
    lui $s5, 0x8001
    ori $s5, $s5, 0x4000
    move $s2, $zero
    move $s3, $zero
    move $s7, $zero
loop:
    lhu $t0, 2($s5)
    nop
    andi $t2, $t0, 0x20
    li $s6, 0x000000ff
    li $t3, 0
    bnez $t2, right_done
    nop
    li $s6, 0x0000ff00
    li $t3, 0x1fff
right_done:
    sh $t3, 0x1c00($s4)
    sh $t3, 0x1c02($s4)
    # CROSS rising edge: reload CD path tables, read the inserted disc ID.
    andi $t2, $t0, 0x4000
    bnez $t2, cross_up
    nop
    bnez $s2, disc_colour
    nop
    li $s2, 1
    # A0:64 is the CD driver chdir(file, path); B0:40 is not chdir.
    move $a0, $zero
    lui $a1, 0x8001
    ori $a1, $a1, 0x1060
    li $t1, 0x64
    jal 0xa0
    nop
    blez $v0, failed
    nop
    lui $a0, 0x8001
    ori $a0, $a0, 0x1020
    li $a1, 1
    li $t1, 0x32
    jal 0xb0
    nop
    bltz $v0, failed
    move $s1, $v0
    move $a0, $s1
    lui $a1, 0x8001
    ori $a1, $a1, 0x3800
    li $a2, 2048
    li $t1, 0x34
    jal 0xb0
    nop
    li $t0, 2048
    bne $v0, $t0, failed
    nop
    move $a0, $s1
    li $t1, 0x36
    jal 0xb0
    nop
    lui $t0, 0x8001
    ori $t0, $t0, 0x3800
    lbu $s3, 0($t0)
    nop
disc_colour:
    li $s6, 0x00ff0000
    li $t0, 2
    beq $s3, $t0, draw
    nop
    li $s6, 0x0000ffff
    b draw
    nop
cross_up:
    move $s2, $zero
draw:
    beq $s6, $s7, loop
    nop
    move $s7, $s6
    lui $t0, 0x0200
    or $t0, $t0, $s6
    sw $t0, 0x1810($s4)
    sw $zero, 0x1810($s4)
    lui $t0, 0x00f0
    ori $t0, $t0, 0x0140
    sw $t0, 0x1810($s4)
    b loop
    nop
failed:
    # White is an observable failure, never a passing gameplay colour.
    lui $s4, 0x1f80
    lui $t0, 0x03ff
    ori $t0, $t0, 0xffff
    sw $t0, 0x1814($s4)
    lui $t0, 0x02ff
    ori $t0, $t0, 0xffff
    sw $t0, 0x1810($s4)
    sw $zero, 0x1810($s4)
    lui $t0, 0x00f0
    ori $t0, $t0, 0x0140
    sw $t0, 0x1810($s4)
halt:
    b halt
    nop
.org 0x1000
.asciz "bu00:SLOT_ACCEPT"
.org 0x1020
.asciz "cdrom:\\DISC.BIN;1"
.org 0x1060
.asciz "cdrom:\\"

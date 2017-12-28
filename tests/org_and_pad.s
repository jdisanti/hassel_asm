.org $E000

;
; Copy the "program" to $A000 and execute it
;
entry:
    LDX #0
copy:
    LDA copy_location, X
    STA $A000, X
    INX
    BNE copy
done:
    JMP $A000

halt:
    JMP halt

;
; Program to copy that expects to be run from $A000
;
copy_location:
.org $A000

program_to_copy:
    LDX #3
pointless_loop:
    DEX
    BNE pointless_loop
    JMP program_to_copy

;
; Vectors
;
.pad $FFFA
.vector halt
.vector entry
.vector halt
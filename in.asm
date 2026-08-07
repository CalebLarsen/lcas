; Count the number of non-negative values in an array

       .ORIG x0200

START
; START OF CODE
;         AND R7, R7, #0
;         LEA R0, Arr
;         LD R1, ArrSz
; LOOP    ADD R1, R1, #-1 
;         BRn LOOP_END
;         ADD R2, R0, R1
;         LDR R2, R2, #0
;         BRn LOOP
;         ADD R7, R7, #1
;         BR LOOP
; 
; Clear Variables
        AND R3, R3, #0
        LEA R0, Arr
        LD R1, ArrSz
        BRnz LOOP_END
        ADD R0, R0, R1
LOOP
         ADD R0, R0, -1
         LDR R2, R0, #0
         BRn PAST_INC
         ADD R3, R3, #1

PAST_INC ADD R1, R1, #-1
         BRp LOOP
        
LOOP_END ST R3, Count
; END OF CODE
BRnzp START     ; once is never enough...

Count  .BLKW #1        ; result goes here
ArrSz  .FILL #10       ; number of array elements
Arr    .FILL #12       ; array data
       .FILL #-23
       .FILL #0
       .FILL x42
       .FILL #17
       .FILL b1111000011110000
       .FILL x59
       .FILL #-76
       .FILL #45
       .FILL xABCD

       .END


.ORIG x0200

        AND R1, R1, R1
        ADD R2, R1, #1
        ST R2, RANGO

RANGO        .FILL #10
JANGO        .STRINGZ "Test2line"

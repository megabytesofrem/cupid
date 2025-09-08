# Opcode Table

| OPCODE | MNEMONIC | OPERAND SIZE | OPERAND TYPE  | DESC                          |
| ------ | -------- | ------------ | ------------- | ----------------------------- |
| 00     | nop      |              |               | no-op                         |
| 01     | push8    | byte         | imm unsigned  | push immediate value          |
| 02     | push16   | word         | imm unsigned  | push immediate value          |
| 03     | push32   | dword        | imm unsigned  | push immediate value          |
| 04     | pushsz   | until_null   | stringz       | push NUL terminated string    |
| 05     | pushac   |              |               | push accumulator              |
|        |          |              |               |                               |
| 06     | pop8     | >byte        |               | pop value from stack          |
| 07     | pop16    | >word        |               | pop value from stack          |
| 08     | pop32    | >dword       |               | pop value from stack          |
| 09     | popsz    | >dword       |               | pop value from stack (string) |
|        |          |              |               |                               |
|        |          |              |               |                               |
| 0C     | cmp      |              |               | pop two values, set f         |
| 0D     | j        | dword        | addr          | jump unconditionally          |
| 0E     | j        | dword        | relative addr | jump unconditionally          |
| 0F     | jeq      | dword        | addr          | jump if f set                 |
| 10     | jne      | dword        | addr          | jump if f not set             |
|        |          |              |               |                               |
| 11     | add      |              |               | pop two values, ze, add, push |
| 12     | sub      |              |               | pop two values, ze, sub, push |
| 13     | mul      |              |               | pop two values, ze, mul, push |
| 14     | div      |              |               | pop two values, ze, mul, push |
|        |          |              |               |                               |
|        |          |              |               |                               |
|        |          |              |               |                               |
| 15     | call     | dword        | addr          | call a function               |
| 16     | callnat  | dword        | name          | call a native function        |
|        |          |              |               |                               |
| FF     | halt     |              |               | halt execution                |
|        |          |              |               |                               |

NOTE: >dword means we return a dword
NOTE: ze means zero-extend, we pad the number so it is 4 bytes long (dword)

## Comparisons

`cmp` works by popping the top two values off the stack, and comparing them and setting `f`.

- `a == b`: `f = 0`
- `a > b`: `f = 1`
- `a < b`: `f = -1`

## Calls

`call` pushes a call frame to the VMs call stack, which includes the return address and
the base pointer.

The return address points to the address after the `call` instruction, so we can jump past
the call site when we are done.

## Disasm format
```
0000: 01 12 : push8 18      02 34 12 : push16 4660    
Stack (sp=0):
  [0]: 18
  [1]: 4660
------------------------------------------------------------
ip: 00000006    sp: 00000000    ac: 00000000    bp: 00000000
------------------------------------------------------------
0000: 01 12 : push8 18      02 34 12 : push16 4660    
Stack (sp=0):
  [0]: 18
  [1]: 4660


```
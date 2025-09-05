'''
Extremely simple assembler for Cupids bytecode
Only supports instructions and labels

NOTE: This will be rewritten in Rust at a later date.
'''
from enum import Enum
import re
import sys

class Op(Enum):
    NOP = 0x00
    PUSHI = 0x01
    PUSH_SZ = 0x02
    POPI = 0x03
    POP_SZ = 0x04
    JMP_ABS = 0x08
    JMP_REL = 0x09
    JEQ = 0x0A
    JNE = 0x0B
    ADD = 0x0C
    SUB = 0x0D
    MUL = 0x0E
    DIV = 0x0F
    HALT = 0xFF

class OperandType(Enum):
    INT = 1

    # String or raw bytes
    STRING_OR_BYTES = 2

class Assembler:
    def __init__(self):
        self.labels = {}
        self.output = []

    def parse_line(self, line):
        line = line.strip()

        if line.startswith('//') or not line:
            return  # Ignore comments
        
        if line.endswith(':'):
            # We handled this in the first pass, ignore
            return 

        self.output.append(self.parse_instruction(line))

    def normalize_op_name(self, op_name):
        match op_name.lower():
            case 'jmp': return 'JMP_ABS'
            case 'jeq': return 'JMP_EQ'
            case 'jne': return 'JMP_NE'
            case 'add': return 'ADD'
            case 'sub': return 'SUB'
            case 'mul': return 'MUL'
            case 'div': return 'DIV'
            case _    : return op_name.upper()

    def parse_instruction(self, line):
        # Parse 'mul', 'jmp <addr>'
        match = re.match(r'(\w+)(?:\s+(.+))?', line)
        # print(match, line)
        if not match:
            raise ValueError(f'Invalid instruction: {line}')

        # TODO: Expand to support more than one operand
        op_name = match.group(1).upper()
        operand = match.group(2) if match.group(2) else None

        normalized_op_name = self.normalize_op_name(op_name)
        translated_operand = None

        # Handle jump specially
        if op_name.lower() == 'jmp':
            # jmp to label, otherwise <address>
            if operand in self.labels:
                translated_operand = self.labels[operand]
            elif operand and any(c.isdigit() for c in operand):
                translated_operand = int(operand, 16)
            else:
                raise ValueError(f'Invalid operand: {operand}')

        elif operand is not None:
            if any(c.isdigit() for c in operand):
                translated_operand = int(operand, 16)
            else:
                translated_operand = operand.encode()

        op = Op[normalized_op_name]
        print(f'{normalized_op_name} → {op.value:02X} {translated_operand if translated_operand is not None else ''}')
        if operand is not None:
            return (op.value, translated_operand)
        return (op.value,)
    
    def assemble(self, code):
        self.labels = {}
        self.output = []

        # First pass: collect labels
        lines = code.splitlines()

        address = 0x0
        for line in lines:
            line = line.strip()
            if line.startswith('//') or not line:
                continue

            if line.endswith(':'):
                self.labels[line[:-1]] = address
            else:
                address += 2 if re.match(r'\w+\s+.+', line) else 1  # crude: 2 bytes for op+operand, 1 for op only

        # Second pass, assemble instructions
        for line in lines:
            self.parse_line(line)

        # Flatten output
        flat_output = []
        for item in self.output:
            flat_output.extend(item)

        return bytes(flat_output)
    
    
def hex_dump(byte_list):
    hex_parts = []
    for val in byte_list:
        if isinstance(val, int):
            hex_parts.append(f'{val:02X}')
        elif isinstance(val, (bytes, bytearray)):
            hex_parts.extend(f'{b:02X}' for b in val)
        else:
            hex_parts.append(str(val))
    return ' '.join(hex_parts)

if __name__ == '__main__':
    asm_file = open(sys.argv[1], 'r')
    code = asm_file.read()
    asm_file.close()

    assembler = Assembler()
    bytecode = assembler.assemble(code)

    print('Output: ')
    print('================================')
    print(hex_dump(bytecode))
    print('================================')

    if sys.argv[2]:
        print('Writing bytecode to ', sys.argv[2])
        output_file = open(sys.argv[2], 'wb')
        output_file.write(bytecode)
        output_file.close()

'''

VM output:
0000: 01 : pushi 4    04 : $04    01 : pushi 2    02 : pushsz '12'    0C : $0C    03 : $03    08 : jmp $0008    08 : jmp $0008    
0008: 08 : jmp $00FF    FF : $FF    00 : $00    00 : $00    00 : $00    00 : $00    00 : $00    00 : $00    
0010: 00 : $00    00 : $00    00 : $00    00 : $00    00 : $00    00 : $00    00 : $00    00 : $00    
0018: 00 : $00    00 : $00                   

Bytecode:

01 04 01 02 0C 03 08 08 08 FF

--
pushi 4 // 01 04
pushi 2 // 01 02
add     // 0C
popi    // 03
jmp $08 // 08 08
jmp $FF // 08 FF

'''
'''
Extremely simple assembler for Cupids bytecode
Only supports instructions and labels

NOTE: This will be rewritten in Rust at a later date.
'''
from enum import Enum
import os
import re
import sys

class Op(Enum):
    NOP = 0x00
    PUSHI = 0x01
    PUSHSZ = 0x02
    POPI = 0x03
    POPSZ = 0x04
    JMP_ABS = 0x08
    JMP_REL = 0x09
    JMP_EQ = 0x0A
    JMP_NE = 0x0B
    ADD = 0x0C
    SUB = 0x0D
    MUL = 0x0E
    DIV = 0x0F
    CALL = 0x10
    CALLNAT = 0x11
    RET = 0x12
    HALT = 0xFF

class OperandType(Enum):
    INT = 1

    # String or raw bytes
    STRING_OR_BYTES = 2

class Assembler:
    def __init__(self, root_dir):
        self.root_dir = root_dir
        self.in_rep = False
        self.rep_count = 0 # reset after each rep
        self.rep_buffer = []
        self.labels = {}

        # TODO: Add data variable section
        self.in_data_section = False
        self.data = {}
        self.output = []

    def parse_line(self, line):
        line = line.strip()

        if line.startswith('//') or not line:
            return  # Ignore comments

        if self.in_rep and not line.startswith('%endrep'):
            self.rep_buffer.append(line)
            return

        if line.startswith('%'):
            self.parse_directive(line)
            return

        if line.endswith(':'):
            # We handled this in the first pass, ignore
            return 

        if self.in_data_section and not line.startswith('%enddata'):
            if ':' in line:
                label_name = line.split(':')[0].strip()
                label_content = line.split(':')[1].strip()
                
                if label_content.startswith('%string'):
                    m = re.match(r'%string\s+"([^"]+)"', label_content)
                    if m:
                        string_value = m.group(1).encode() + b'\x00'
                        self.data[label_name] = list(string_value)
                elif label_content.startswith('%bytes'):
                    m = re.match(r'%bytes\s+(.+)', label_content)
                    if m:
                        byte_values = [int(x, 16) for x in m.group(1).split(' ')]
                        self.data[label_name] = byte_values
                else:
                    try:
                        byte_values = [int(x, 16) for x in label_content.split(' ')]
                        self.data[label_name] = byte_values
                    except ValueError:
                        pass  # Not hex bytes, ignore
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

    def define_data_label(self, label, value):
        if label in self.data:
            raise ValueError(f'Duplicate data label: {label}')

        self.data[label] = value

    def parse_directive(self, line):
        import os.path

        match = re.match(r'%(\w+)(?:\s+(.+))?', line)
        if not match:
            raise ValueError(f'Invalid directive: {line}')

        directive = match.group(1).lower()
        operand = match.group(2)

        match directive:
            case 'include':
                # Handle include directives
                file_path = operand.strip('"')
                include_path = os.path.join(self.root_dir, file_path)
                print(f"Path: {file_path}, relative: {include_path}")

                print(f"DEBUG: Directive 'include {file_path}'")
                with open(include_path, 'r') as f:
                    included_code = f.read()
                    for inc_line in included_code.splitlines():
                        self.parse_line(inc_line)
            case 'rep':
                self.in_rep = True
                self.rep_count = int(operand)
            case 'endrep':
                self.in_rep = False
                for _ in range(self.rep_count):
                    for rep_line in self.rep_buffer:
                        self.parse_line(rep_line)

                self.rep_buffer = []
                self.rep_count = 0
            case 'data':
                self.in_data_section = True
            case 'enddata':
                self.in_data_section = False
            case 'bytes':
                byte_values = [int(x, 16) for x in operand.split(' ')]
                print(f"DEBUG: Directive 'bytes {' '.join(f'{b:02X}' for b in byte_values)}'")
                self.output.append(tuple(byte_values))
            case 'string':
                byte_values = operand.strip('"').encode()  # null-terminated
                self.output.append((byte_values, 0))

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

        op = Op[normalized_op_name]
        address = 0x0

        # Handle jump specially
        if op_name.lower() == 'jmp':
            # jmp to label, otherwise <address>
            if operand in self.labels:
                address = self.labels[operand]
            elif operand and any(c.isdigit() for c in operand):
                address = int(operand, 16)
            else:
                raise ValueError(f'Invalid operand: {operand}')
            

            # Generate bytes without leading zeros
            if address == 0:
                return (op.value, 0)  # Just the null terminator for address 0
            else:
                bytes_needed = []
                temp = address
                while temp > 0:
                    bytes_needed.insert(0, temp & 0xFF)
                    temp >>= 8
                return (op.value, *bytes_needed, 0)


        if operand is not None:
            if any(c.isdigit() for c in operand):
                translated_operand = int(operand, 16)
            else:
                if isinstance(operand, str) and (operand.startswith('"') and operand.endswith('"')):
                    string_literal = operand.strip('"')
                    string_bytes = string_literal.encode() + b'\x00'  # null-terminated
                    return (op.value, *string_bytes, 0)
                else:
                    if operand.startswith("$"):
                        label = operand[1:]
                        # Return the data
                        print(f'DATA: {label} {self.data}')
                        if label in self.data:
                            return (op.value, *self.data[label])
                        else:
                            raise ValueError(f'Undefined label: {label}')
                    else:
                        translated_operand = operand.encode() + b'\x00'  # null-terminated

        print(f'{normalized_op_name} → {op.value:02X} {translated_operand if translated_operand is not None else ''}')
        if operand is not None:
            return (op.value, translated_operand)
        return (op.value,)
    
    def assemble(self, code):
        self.labels = {}
        self.output = []
        self.data = {}  # Reset data

        # First pass: collect labels
        lines = code.splitlines()

        address = 0x0
        in_data_section = False
        
        for line in lines:
            line = line.strip()
            if line.startswith('//') or not line:
                continue
        
            if line.startswith('%'):
                match = re.match(r'%(\w+)(?:\s+(.+))?', line)  # Make operand optional
                if match:
                    directive = match.group(1).lower()
                    if directive == 'data':
                        in_data_section = True
                    elif directive == 'enddata':
                        in_data_section = False
                    elif directive == 'bytes' and match.group(2):
                        byte_count = len(match.group(2).split(' '))
                        address += byte_count
                continue

            if line.endswith(':'):
                label_name = line[:-1]
                self.labels[label_name] = address
                print(f"Label '{label_name}' at address 0x{address:04X}")
            elif in_data_section and ':' in line:
                # Handle data labels like "msg: %string 'hello'" - DON'T count towards address
                label_name = line.split(':')[0].strip()
                self.labels[label_name] = address
                print(f"Data label '{label_name}' at address 0x{address:04X}")
            else:
                # Only count instruction sizes if NOT in data section
                if not in_data_section:
                    if re.match(r'\w+\s+.+', line):
                        if line.strip().startswith('j'):
                            address += 4 # opcode + address
                        else:
                            address += 2 # opcode + operand
                    else:
                        address += 1 # opcode only

        print(f"Final labels: {self.labels}")
        print(f"Final data: {self.data}")

        # Second pass: assemble instructions
        for line in lines:
            self.parse_line(line)

        # Flatten output
        flat_output = []
        for item in self.output:
            if isinstance(item, tuple):
                for subitem in item:
                    if isinstance(subitem, bytes):
                        flat_output.extend(subitem)
                    else:
                        flat_output.append(subitem)
            else:
                if isinstance(item, bytes):
                    flat_output.extend(item)
                else:
                    flat_output.append(item)

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

    root_dir = os.path.dirname(os.path.abspath(sys.argv[1]))

    assembler = Assembler(root_dir)
    bytecode = assembler.assemble(code)

    print('Output: ')
    print('================================')
    print(hex_dump(bytecode))
    print('================================')

    if sys.argv[2]:
        print('Writing bytecode to', sys.argv[2])
        output_file = open(sys.argv[2], 'wb')
        output_file.write(bytecode)
        output_file.close()

'''

Directives
-------------
%include filename - cut and paste the file contents
%bytes 0x01 0x02 0x03 - cut and paste byte values
%string "hello" - cut and paste a string, nul-terminated
%rep count <lines> %endrep - repeat the enclosed lines 'count' times

Data section
------------
This was a *nightmare* to get working, genuinely almost cried.

Conventions
-------------
1. Use labels as a form of variable and %string to splat bytes, $ will extract their bytes
msg:
  %string "hello"
pushsz $msg

2. Use %rep to avoid rewriting the same sequence of instructions.

3. Use %data to define variables, there can only be ONE data section.
%data
  msg: %string "hello"
  len: %bytes 0x05
%enddata

'''
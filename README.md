# Cupid

A small programming language targeting its own stack VM. 💘

## Syntax Example:

```
func main()
  val result = 1 + 2
  print(to_string(result))
end
```

## Assembly

To save having to type out bytes while testing the VM, I wrote a extremely rudimentary assembler.

### Comments

Comments are C style single line comments. They cannot be anywhere other than
the start of a line

### Directives

- `%include filename` - cut and paste the file contents
- `%bytes 0x01 0x02 0x03` - cut and paste byte values
- `%string "hello"` - cut and paste a string, nul-terminated
- Repetition using `rep`, same syntax as NASM

  ```armasm
  %rep 5
    pushi 1
  %endrep
  ```

- Data section, there can only be one data section defined. Everything is globally scoped.

  ```armasm
  %data
    msg: %string "hello world"
    // or len: 0x11
    len: %bytes 0x11
  %enddata

  // use $ to reference a label defined in a data section
  // this will grab the value assigned to it, *not* the address
  pushsz $msg
  pushi $len
  callnat print
  ```

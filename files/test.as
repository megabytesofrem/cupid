pushi 4
pushi 2
add
popi

//pushsz "hello"

// push 5, discarding 6 from the stack
pushi 5
//call 1


// jump to label done, which then jumps to 0xFF
jmp done
done:
  jmp 0xFF
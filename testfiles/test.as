start:

j another
another: 
  pushsz "hello"
  popsz
  pushsz "world"
  j 0xcafebabe


  // %rep(5)
  // nop
  // %endrep

// pushi 5
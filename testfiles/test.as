start:

j another
another: 
  pushsz "hello"
  popsz
  pushsz "world"
  // pushbz [0x01 0x02]
  //j exit

exit:

  // %rep(5)
  // nop
  // %endrep

// pushi 5
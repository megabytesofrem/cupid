// comment
start:

%data
  msg: %string "hello"
%enddata

// push 1 and 2
pushsz $msg

%rep 255
  pushi 1
%endrep
// pushi $len

// %include "test2.as"
// comment
part1:
  pushi 01

%data
  msg: %string "hello"
  len: %bytes 0x05
%enddata

pushsz $msg
pushi $len

%include "test2.as"
%include "test2.as"
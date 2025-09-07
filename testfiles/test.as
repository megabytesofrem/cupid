%define MSG "Hello\0"
%define COUNT 5
%define BYTES [0x41 0x41 0x41 0x42 0]

%rep(5)
  pushsz MSG
  pushsz BYTES
%endrep

%include "test2.as"

j start
start:

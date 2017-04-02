A MSP430 project that plays music using PWM and prints out its own source code.

The Rust project is used to generate the code to store the source code with some
rudimentary compression; currently it uses a combination of what I read about
the LZ77 algorithm from Wikipedia along with Huffman encoding. The python script
repeatedly runs the compression and code injection program over the source code
until it reaches a fixed point where the source code stops changing. In the current
setup, this will almost always be in three iterations. The Rust program
takes the source code in stdin and outputs the altered version in stdout.

TODOS
- the music part
- fix extra generated newline

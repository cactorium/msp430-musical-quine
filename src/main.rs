#[derive(Clone, Copy, Debug)]
enum Symbol {
    Literal(u8),
    Offset(u8),
}

struct RingBuffer {
    window: [u8; 256], // buffer
    len: usize, // number of valid bytes in the buffer
    offset: usize // location of the next byte
}

struct RingBufferIter<'a> {
    me: &'a RingBuffer,
    pos: usize
}

impl <'a> Iterator for RingBufferIter<'a> {
    type Item = &'a u8;
    fn next(&mut self) -> Option<&'a u8> {
        if self.pos >= self.me.len {
            None
        } else {
            let ret = Some(&self.me.window[(256 - self.me.len + self.me.offset + self.pos) % 256]);
            self.pos += 1;
            ret
        }
    }
}


impl RingBuffer {
    fn new() -> RingBuffer {
        RingBuffer { window: [0; 256], len: 0, offset: 0 }
    }

    fn offset<'a>(&'a self, off: usize) -> Option<RingBufferIter<'a>> {
        if off <= self.len {
            Some(RingBufferIter {
                me: self,
                pos: self.len - off
            })
        } else {
            None
        }
    }

    fn push(&mut self, s: &[u8]) {
        for (i, c) in s.iter().enumerate() {
            self.window[(self.offset + i) % 256] = *c;
        }
        self.len = self.len + s.len();
        if self.len > 256 {
            self.len = 256;
        }
        self.offset = (self.offset + s.len()) % 256;
    }

    fn push_u8(&mut self, c: u8) {
        self.window[self.offset] = c;
        self.len = self.len + 1;
        if self.len > 256 {
            self.len = 256;
        }
        self.offset = (self.offset + 1) % 256;
    }

    fn prev(&self, s: usize) -> Option<u8> {
        if s < self.len {
            Some(self.window[(256 + self.offset - s) % 256])
        } else {
            None
        }
    }
}

fn lz77_enc(input: &[u8]) -> Vec<Symbol> {
    // FIXME: replace window with a iterable ring buffer
    let mut window = RingBuffer::new();
    let mut ret = Vec::new();
    let mut i = 0;

    fn search_substr(s: &[u8], window: &RingBuffer) -> Option<(usize, usize)> {
        (0..window.len)
            .map(|i| {
                let mut iter = window.offset(i).unwrap().peekable();
                if !iter.peek().is_none() {
                    // chain s to the iter for run length encoding
                    let len = iter.chain(s.iter()).zip(s.iter())
                        .take_while(|&(x, y)| *x == *y)
                        .count();
                    if len > 0 {
                        Some((i, len))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .filter(|x| x.is_some())
            .map(|x| x.unwrap())
            .max_by(|&(_, x), &(_, y)| x.cmp(&y))
    }

    while i < input.len() {
        match search_substr(&input[i..input.len()], &window) {
            Some((pos, len)) => {
                assert!(pos < 256);
                assert!(len < 256);
                // runs of length less than 4 are probably better encoded
                // as literals; it's hard to be sure because of the Huffman
                // encoding layer on top of this
                if len < 4 {
                    ret.push(Symbol::Literal(input[i]));
                    window.push(&input[i..i+1]);
                    i += 1;
                } else {
                    ret.push(Symbol::Offset(pos as u8));
                    ret.push(Symbol::Literal(len as u8));
                    window.push(&input[i..i+len]);
                    i += len;
                }
            },
            None => {
                ret.push(Symbol::Literal(input[i]));
                window.push(&input[i..i+1]);
                i += 1;
            },
        }
    }

    ret
}

fn lz77_dec(input: &[Symbol]) -> Vec<u8> {
    #[derive(Debug)]
    enum StreamToken {
        Literal(u8),
        OffsetLen(usize, usize)
    }
    let (ret, _) = input
        .iter()
        .zip(input.iter().chain(Some(&Symbol::Literal(0))).skip(1))
        .scan(false, |skip_input, (input, next_input)| {
            if *skip_input {
                *skip_input = false;
                Some(None)
            } else {
                *skip_input = false;
                match input {
                    &Symbol::Literal(c) => {
                        Some(Some(StreamToken::Literal(c)))
                    },
                    &Symbol::Offset(o) => {
                        match next_input {
                            &Symbol::Literal(l) => {
                                *skip_input = true;
                                Some(Some(StreamToken::OffsetLen(o as usize, l as usize)))
                            }
                            &Symbol::Offset(_) => {
                                // parse failure; I guess try to continue?
                                Some(None)
                            },
                        }
                    },
                }
            }
        })
        .filter(|x| x.is_some())
        .map(|x| x.unwrap())
        .fold((Vec::new(), RingBuffer::new()), |(mut output, mut window), stream_token| {
            match stream_token {
                StreamToken::Literal(c) => {
                    output.push(c);
                    window.push_u8(c);
                    (output, window)
                },
                StreamToken::OffsetLen(o, l) => {
                    for _ in 0..l {
                        // FIXME: treat as parse error instead
                        let c = window.prev(o).unwrap();
                        output.push(c);
                        window.push_u8(c);
                    }
                    (output, window)
                }
            }
        });
    ret
}

#[test]
fn test_lz77() {
    use std::str;
    {
        let test_str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let test_stream = lz77_enc(test_str.as_bytes());
        println!("{:?}", test_stream);
        let test_dec = lz77_dec(&test_stream);
        assert_eq!(test_str, str::from_utf8(&test_dec).unwrap());
    }
    {
        let test_str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaabbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbaaaaaaaaaaaaabababasdckagsakjdfhkwajrhglkjsadvlkjahgirwarraaagaoabrwaalaaaabababababbbaba";
        let test_stream = lz77_enc(test_str.as_bytes());
        println!("{:?}", test_stream);
        let test_dec = lz77_dec(&test_stream);
        assert_eq!(test_str, str::from_utf8(&test_dec).unwrap());
    }
}

struct Huffman {
}

fn huffman_dict(input: Vec<Symbol>) -> Huffman {
    unimplemented!()
}

fn main() {
    use std::io;
    use std::io::Read;

    let mut string = String::new();
    let mut input = io::stdin().read_to_string(&mut string);

    let stream = lz77_enc(string.as_bytes());
    println!("// orig {} stream {}", string.len(), stream.len());
    println!("// checking decode...");
    let dec_string = String::from_utf8(lz77_dec(&stream)).unwrap();
    if string == dec_string {
        println!("// decode successful");
    } else {
        println!("// decode fail");
    }
}

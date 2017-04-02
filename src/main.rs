use std::collections::HashMap;
use std::hash::Hash;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Symbol {
    Literal(usize),
    Offset(usize),
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
                // assert!(len < 256);
                // runs of length less than 4 are probably better encoded
                // as literals; it's hard to be sure because of the Huffman
                // encoding layer on top of this
                if len < 4 {
                    ret.push(Symbol::Literal(input[i] as usize));
                    window.push(&input[i..i+1]);
                    i += 1;
                } else {
                    ret.push(Symbol::Offset(pos));
                    ret.push(Symbol::Literal(len));
                    window.push(&input[i..i+len]);
                    i += len;
                }
            },
            None => {
                ret.push(Symbol::Literal(input[i] as usize));
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
                        Some(Some(StreamToken::Literal(c as u8)))
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

#[derive(Debug)]
enum Huffman<S> {
    Branch(Rc<Huffman<S>>, Rc<Huffman<S>>),
    Leaf(S),
}

#[derive(Debug)]
struct HuffmanDict<S> {
    root: Rc<Huffman<S>>
}

fn huffman_dict<S: Eq + Hash + Copy>(input: &[S]) -> HuffmanDict<S> {
    let mut dict: HashMap<S, usize> = HashMap::new();
    let mut symbol_list = Vec::new();
    for i in input {
        let entry = dict.entry(*i).or_insert_with(|| {symbol_list.push(*i); 0});
        *entry += 1;
    }
    let mut leaves: Vec<(usize, Rc<Huffman<S>>)> = symbol_list
        .iter()
        .map(|sym| {
            (dict[sym], Rc::new(Huffman::Leaf(*sym)))
        })
        .collect();
    while leaves.len() >= 2 {
        leaves.sort_by_key(|&(w, _)| -(w as isize));
        let ((w0, a), (w1, b)) = (leaves.pop().unwrap(), leaves.pop().unwrap());
        let new_node = Rc::new(Huffman::Branch(a, b));
        leaves.push((w0 + w1, new_node));
    }
    let (_, root) = leaves.pop().unwrap();
    HuffmanDict {
        root: root
    }
}

fn huffman_enc<S: Eq + Hash + Copy>(input: &[S]) -> (Vec<u8>, usize, HuffmanDict<S>) {
    let dict = huffman_dict(input);
    let mut mapping: HashMap<S, Vec<bool>> = HashMap::new();
    {
        fn recursively_map<T: Eq + Hash + Copy>(prefix: Vec<bool>, node: &Huffman<T>, mapping: &mut HashMap<T, Vec<bool>>) {
            match node {
                &Huffman::Branch(ref a, ref b) => {
                    let mut prefixa = prefix.clone();
                    let mut prefixb = prefix;
                    prefixa.push(false);
                    prefixb.push(true);
                    recursively_map(prefixa, &*a, mapping);
                    recursively_map(prefixb, &*b, mapping);
                },
                &Huffman::Leaf(sym) => {
                    mapping.insert(sym, prefix);
                },
            }
        }
        recursively_map(Vec::new(), &*dict.root, &mut mapping);
    }

    let mut ret = Vec::<u8>::new();
    let mut len = 0;

    {
        for s in input {
            for bit in &mapping[s] {
                if len % 8 == 0 {
                    ret.push(0);
                }
                if *bit {
                    *ret.last_mut().unwrap() |= 1 << (len % 8);
                }
                len += 1;
            }
        }
    }
    (ret, len, dict)
}

fn huffman_dec<S: Copy>(input: &[u8], len: usize, dict: &HuffmanDict<S>) -> Vec<S> {
    let mut pos = 0;
    let mut dict_pos = &*dict.root;
    let mut ret = Vec::new();
    while pos < len {
        match dict_pos {
            &Huffman::Leaf(sym) => {
                ret.push(sym);
                dict_pos = &*dict.root;
            },
            &Huffman::Branch(ref a, ref b) => {
                let bit = input[pos/8] & (1 << (pos % 8));
                pos += 1;

                if bit == 0 {
                    dict_pos = &*a;
                } else {
                    dict_pos = &*b;
                }
            },
        }
    }

    match dict_pos {
         &Huffman::Leaf(sym) => {
            ret.push(sym);
         },
         _ => unreachable!("decoder reached EOS in wrong state"),
    }
    ret
}

/*
fn huffman_dec_c(input: &HuffmanDict<Symbol>) -> String {
    let mut s = String::from("int pos = 0;\n");
    s += "void dec() {\n";
    s += "  while (pos < len) {\n";
    fn recursively_if(s: &mut String, pos: &Huffman<Symbol>, indent: usize) {
        let indent_stuff = |s: &mut String, x| {
            for _ in 0..x {
                *s += "  ";
            }
        };

        match pos {
            &Huffman::Leaf(sym) => {
                indent_stuff(s, indent);
                match sym {
                    Symbol::Literal(c) => {
                        *s += format!("LITERAL({});\n", c).as_str();
                    },
                    Symbol::Offset(o) => {
                        *s += format!("LEN({});\n", o).as_str();
                    }
                }
            },
            &Huffman::Branch(ref a, ref b) => {
                indent_stuff(s, indent);
                *s += "if (bit) {\n";
                indent_stuff(s, indent + 1);
                *s += "NEXT_BIT;\n";
                recursively_if(s, &*b, indent + 1);
                indent_stuff(s, indent);
                *s += "} else {\n";
                indent_stuff(s, indent + 1);
                *s += "NEXT_BIT;\n";
                recursively_if(s, &*a, indent + 1);
                indent_stuff(s, indent);
                *s += "}\n";
            },
        }
    }
    recursively_if(&mut s, &*input.root, 2);
    s += "  }\n";
    s += "}";
    s
}
*/

fn c_lit(chars: &[u8]) -> String {
    let mut s = String::from("{");
    for c in chars {
        s += format!("{}, ", *c).as_str();
    }
    s += "0}"; // fuck it, we're generating C anyways
    s
}

fn c_lit_int(chars: &[isize]) -> String {
    let mut s = String::from("{");
    for c in chars {
        s += format!("{}, ", *c).as_str();
    }
    s += "0}"; // fuck it, we're generating C anyways
    s
}

fn huffman_dict_c(dict: &HuffmanDict<Symbol>) -> Vec<isize> {
    let mut ret = Vec::new();
    fn recursively_traverse(buf: &mut Vec<isize>, node: &Huffman<Symbol>) -> usize {
        let pos = buf.len();
        match node {
            &Huffman::Branch(ref a, ref b) => {
                // push some temporary values to rewrite later
                buf.push(0);
                buf.push(0);
                let aidx = recursively_traverse(buf, &*a);
                let bidx = recursively_traverse(buf, &*b);
                buf[pos] = aidx as isize;
                buf[pos+1] = bidx as isize;
            },
            &Huffman::Leaf(sym) => {
                match sym {
                    Symbol::Literal(c) => {
                        buf.push(-(c as isize));
                    },
                    Symbol::Offset(o) => {
                        buf.push(-((o + 256) as isize));
                    }
                }
            }
        }
        pos
    }

    let _ = recursively_traverse(&mut ret, &*dict.root);
    ret
}

fn huffman_dec_f() -> String {
    // TODO
    String::from("")
}

fn main() {
    use std::io;
    use std::io::{Read, Write};
    let stderr = &mut io::stderr();

    let mut string = String::new();
    io::stdin().read_to_string(&mut string).unwrap();

    let string_parsed = {
        use std::iter;
        string.lines()
            .zip(iter::once("").chain(string.lines()))
            .map(|(x, prev)| {
                let replace_str = "//+replace ";
                if prev.starts_with(replace_str) {
                    &prev[replace_str.len()..]
                } else {
                    x
                }
            })
            .fold(String::new(), |mut x, y| { x += y; x += "\n"; x })
    };

    let stream = lz77_enc(string_parsed.as_bytes());
    writeln!(stderr, "// orig {} 77z {}", string.len(), stream.len()).unwrap();
    let (huffman_enc, huffman_len, dict) = huffman_enc(&stream);
    // writeln!(stderr, "// {:?}", dict).unwrap();
    writeln!(stderr, "// huffman {} bits, {} bytes", huffman_len, (huffman_len + 7)/8).unwrap();

    writeln!(stderr, "// checking decode...").unwrap();
    let dec_string = String::from_utf8(lz77_dec(&huffman_dec(&huffman_enc, huffman_len, &dict))).unwrap();
    if string_parsed == dec_string {
        writeln!(stderr, "// decode successful").unwrap();
        // writeln!(stderr, "{}", dec_string).unwrap();
    } else {
        writeln!(stderr, "// decode fail").unwrap();
        writeln!(stderr, "{:?}\n{:?}", string, dec_string).unwrap();
    }
    let mut autogen_start = false;
    for line in string.lines() {
        if !autogen_start {
            println!("{}", line);
            autogen_start = line.starts_with("///AUTOGEN START");
        }
    }
    let huff_dict_ary = huffman_dict_c(&dict);
    writeln!(stderr, "// huffman dict {} entries, {} bytes", huff_dict_ary.len(), 2*huff_dict_ary.len()).unwrap();
    // stop from trying to encode the previous code
    println!("//+replace CODE;");
    println!("const uint8_t code[] = {}; const int len = {};", c_lit(&huffman_enc), huffman_len);
    println!("");
    println!("//+replace DICT;");
    println!("const uint16_t huffman[] = {};", c_lit_int(&huff_dict_ary));
    println!("");
    println!("{}", huffman_dec_f());
}

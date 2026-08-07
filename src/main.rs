use std::{collections::HashMap, iter::Peekable, num::ParseIntError, vec::IntoIter};

fn parse_num(s: &str) -> Result<i16, ParseIntError> {
    if let Some(num) = s.strip_prefix("0x") {
        u16::from_str_radix(num, 16).map(|i| i as i16)
    } else if let Some(num) = s.strip_prefix("x") {
        u16::from_str_radix(num, 16).map(|i| i as i16)
    } else if let Some(num) = s.strip_prefix("b") {
        u16::from_str_radix(num, 2).map(|i| i as i16)
    } else if let Some(num) = s.strip_prefix("#") {
        num.parse::<i16>()
    } else {
        s.parse::<i16>()
    }
}

#[derive(Debug)]
enum LexerError {
    ParseIntError,
    InvalidChar(char),
    InvalidPseudoInst(String),
    InvalidString,
    InputFinished,
    BadBranch,
}

#[derive(Debug)]
enum Token {
    // Pseudo Ops
    Orig,
    Fill,
    Blkw,
    Stringz,
    End,
    // Rest
    Comma,
    Number { value: i16 },
    Register { num: u8 },
    Label { name: String },
    String(String),
    // Ops
    Add,
    And,
    Br { n: bool, p: bool, z: bool },
    Jmp,
    Jsr,
    Jsrr,
    Ld,
    Ldi,
    Ldr,
    Lea,
    Not,
    Ret,
    Rti,
    St,
    Sti,
    Str,
    Trap,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            chars: s.chars().peekable(),
        }
    }

    fn get_word(&mut self, start: Option<char>) -> String {
        let mut out = String::new();
        if let Some(c) = start {
            out.push(c);
        }
        while let Some(&peeked) = self.chars.peek() {
            if peeked.is_ascii_alphanumeric() || peeked == '_' || peeked == '-' {
                out.push(peeked);
                self.chars.next();
            } else {
                break;
            }
        }
        out
    }

    fn get_pseudo(&mut self) -> Result<Token, LexerError> {
        let s = self.get_word(None);
        match s.as_str() {
            "ORIG" => Ok(Token::Orig),
            "BLKW" => Ok(Token::Blkw),
            "FILL" => Ok(Token::Fill),
            "STRINGZ" => Ok(Token::Stringz),
            "END" => Ok(Token::End),
            v => Err(LexerError::InvalidPseudoInst(v.to_string())),
        }
    }

    fn get_string(&mut self) -> Result<String, LexerError> {
        let mut out = String::new();
        while let Some(c) = self.chars.next() {
            if c == '"' {
                return Ok(out);
            } else if c == '\n' {
                return Err(LexerError::InvalidString);
            }
            out.push(c);
        }
        Ok(out)
    }

    fn is_register(&self, s: &str) -> bool {
        matches!(s.as_bytes(), [b'R', b'0'..=b'7'])
    }

    fn is_number(&self, s: &str) -> bool {
        let bytes = s.as_bytes();
        match bytes {
            [b'b', rest @ ..] if !rest.is_empty() => rest.iter().all(|b| matches!(b, b'0' | b'1')),
            [b'x', rest @ ..] if !rest.is_empty() => rest.iter().all(|b| b.is_ascii_hexdigit()),
            [b'0', b'x', rest @ ..] if !rest.is_empty() => {
                rest.iter().all(|b| b.is_ascii_hexdigit())
            }
            [b'#', b'-', rest @ ..] if !rest.is_empty() => rest.iter().all(|b| b.is_ascii_digit()),
            [b'#', rest @ ..] if !rest.is_empty() => rest.iter().all(|b| b.is_ascii_digit()),
            [b'-', rest @ ..] if !rest.is_empty() => rest.iter().all(|b| b.is_ascii_digit()),
            _ if !bytes.is_empty() => bytes.iter().all(|b| b.is_ascii_digit()),
            _ => false,
        }
    }

    fn parse_branch(&mut self, branch: &str) -> Result<Token, LexerError> {
        let n = branch.matches("n").count();
        let z = branch.matches("z").count();
        let p = branch.matches("p").count();
        if n > 1 || z > 1 || p > 1 {
            return Err(LexerError::BadBranch);
        }
        Ok(Token::Br {
            n: n == 1,
            p: p == 1,
            z: z == 1,
        })
    }

    fn parse_words(&mut self, start: char) -> Result<Token, LexerError> {
        let s = self.get_word(Some(start));
        match s.as_str() {
            "ADD" => Ok(Token::Add),
            "AND" => Ok(Token::And),
            "JMP" => Ok(Token::Jmp),
            "JSR" => Ok(Token::Jsr),
            "JSRR" => Ok(Token::Jsrr),
            "LD" => Ok(Token::Ld),
            "LDI" => Ok(Token::Ldi),
            "LDR" => Ok(Token::Ldr),
            "LEA" => Ok(Token::Lea),
            "NOT" => Ok(Token::Not),
            "RET" => Ok(Token::Ret),
            "RTI" => Ok(Token::Rti),
            "ST" => Ok(Token::St),
            "STI" => Ok(Token::Sti),
            "STR" => Ok(Token::Str),
            "TRAP" => Ok(Token::Trap),
            branch if s.starts_with("BR") => self.parse_branch(branch),
            _ => {
                if self.is_register(s.as_str()) {
                    Ok(Token::Register {
                        num: s.chars().nth(1).unwrap() as u8 - b'0',
                    })
                } else if self.is_number(s.as_str()) {
                    match parse_num(s.as_str()) {
                        Ok(value) => return Ok(Token::Number { value }),
                        Err(e) => {
                            println!("Error parsing number of '{:?}': {:?}", s, e);
                            return Err(LexerError::ParseIntError);
                        }
                    }
                } else {
                    return Ok(Token::Label { name: s });
                }
            }
        }
    }

    fn next_token(&mut self) -> Result<Token, LexerError> {
        let mut in_comment = false;
        while let Some(next) = self.chars.next() {
            match next {
                '\n' => in_comment = false,
                _ if in_comment => continue,
                ';' => in_comment = true,
                ' ' | '\t' | '\r' => continue,
                ',' => return Ok(Token::Comma),
                '.' => return self.get_pseudo(),
                '"' => return Ok(Token::String(self.get_string()?)),
                '0'..='9' | 'a'..='z' | 'A'..='Z' | '#' | '-' | '_' => {
                    match self.parse_words(next) {
                        Ok(t) => return Ok(t),
                        Err(_) => return Err(LexerError::ParseIntError),
                    }
                }
                c => return Err(LexerError::InvalidChar(c)),
            }
        }
        Err(LexerError::InputFinished)
    }

    pub fn tokenize(&mut self) -> Result<std::vec::Vec<Token>, LexerError> {
        let mut vec: Vec<Token> = std::vec::Vec::new();
        loop {
            match self.next_token() {
                Ok(t) => {
                    vec.push(t);
                }
                Err(LexerError::InputFinished) => break,
                Err(e) => return Err(e),
            }
        }
        Ok(vec)
    }
}

type Reg = u8;

#[derive(Debug)]
enum Instruction {
    Orig(i16),
    End,
    Fill(i16),
    Blkw(i16),
    Stringz(String),
    AddRR {
        d: Reg,
        s1: Reg,
        s2: Reg,
    },
    AddRI {
        d: Reg,
        s1: Reg,
        imm: i16,
    },
    AndRR {
        d: Reg,
        s1: Reg,
        s2: Reg,
    },
    AndRI {
        d: Reg,
        s1: Reg,
        imm: i16,
    },
    Not {
        d: Reg,
        s1: Reg,
    },
    Ld {
        d: Reg,
        label: String,
    },
    Ldi {
        d: Reg,
        label: String,
    },
    Ldr {
        d: Reg,
        base: Reg,
        imm: i16,
    },
    Lea {
        d: Reg,
        label: String,
    },
    St {
        s1: Reg,
        label: String,
    },
    Sti {
        s1: Reg,
        label: String,
    },
    Str {
        s1: Reg,
        base: Reg,
        imm: i16,
    },
    Br {
        n: bool,
        z: bool,
        p: bool,
        label: String,
    },
    Jmp {
        base: Reg,
    },
    Jsr {
        label: String,
    },
    Jsrr {
        base: Reg,
    },
    Ret,
    Rti,
    Trap {
        trapvect: i16,
    },
}

type SymbolMap = HashMap<String, i16>;

#[derive(Debug)]
enum ParseError {
    UnexpectedToken(String),
}

struct Parser {
    tokens: Peekable<IntoIter<Token>>,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens: tokens.into_iter().peekable(),
        }
    }

    fn expect_number(&mut self) -> Result<i16, ParseError> {
        match self.tokens.next().unwrap() {
            Token::Number { value } => Ok(value),
            t => Err(ParseError::UnexpectedToken(format!(
                "Expected Number, found: {:?}",
                t
            ))),
        }
    }

    fn expect_string(&mut self) -> Result<String, ParseError> {
        match self.tokens.next().unwrap() {
            Token::String(string) => Ok(string),
            t => Err(ParseError::UnexpectedToken(format!(
                "Expected String, found: {:?}",
                t
            ))),
        }
    }

    fn expect_comma(&mut self) -> Result<(), ParseError> {
        match self.tokens.next().unwrap() {
            Token::Comma => Ok(()),
            t => Err(ParseError::UnexpectedToken(format!(
                "Expected Comma, found: {:?}",
                t
            ))),
        }
    }

    fn expect_register(&mut self) -> Result<Reg, ParseError> {
        match self.tokens.next().unwrap() {
            Token::Register { num } => Ok(num),
            t => Err(ParseError::UnexpectedToken(format!(
                "Expected Register, found: {:?}",
                t
            ))),
        }
    }

    fn expect_label(&mut self) -> Result<String, ParseError> {
        match self.tokens.next().unwrap() {
            Token::Label { name } => Ok(name),
            t => Err(ParseError::UnexpectedToken(format!(
                "Expected Label, found: {:?}",
                t
            ))),
        }
    }

    pub fn parse(&mut self) -> Result<(SymbolMap, Vec<Instruction>), ParseError> {
        let mut symbol_map = SymbolMap::new();
        let mut inst = Vec::new();
        let first = self.tokens.next().unwrap();
        match first {
            Token::Orig => (),
            _ => {
                return Err(ParseError::UnexpectedToken(
                    "First token must be .ORIG".to_string(),
                ));
            }
        }
        let addr_tok = self.tokens.next().unwrap();
        let mut addr = match addr_tok {
            Token::Number { value } => value,
            _ => panic!("Expected number after .ORIG"),
        };
        symbol_map.insert(".ORIG".to_string(), addr);
        inst.push(Instruction::Orig(addr));
        while let Some(t) = self.tokens.next() {
            match t {
                Token::Orig => panic!("Expected only 1 .ORIG"),
                Token::Fill => {
                    let val = self.expect_number()?;
                    inst.push(Instruction::Fill(val));
                    addr += 1;
                }
                Token::Blkw => {
                    let val = self.expect_number()?;
                    inst.push(Instruction::Blkw(val));
                    addr += val;
                }
                Token::Stringz => {
                    let s = self.expect_string()?;
                    addr += s.len() as i16 + 1;
                    inst.push(Instruction::Stringz(s));
                }
                Token::End => {
                    inst.push(Instruction::End);
                }
                Token::Label { name } => {
                    symbol_map.insert(name, addr);
                }
                Token::Add => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let s1 = self.expect_register()?;
                    self.expect_comma()?;
                    match self.tokens.next().unwrap() {
                        Token::Number { value } => inst.push(Instruction::AddRI {
                            d: dest,
                            s1,
                            imm: value,
                        }),

                        Token::Register { num } => inst.push(Instruction::AddRR {
                            d: dest,
                            s1,
                            s2: num,
                        }),
                        t => {
                            return Err(ParseError::UnexpectedToken(format!(
                                "Expected Number or register, Found: {:?}",
                                t
                            )));
                        }
                    }
                    addr += 1;
                }
                Token::And => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let s1 = self.expect_register()?;
                    self.expect_comma()?;
                    match self.tokens.next().unwrap() {
                        Token::Number { value } => inst.push(Instruction::AndRI {
                            d: dest,
                            s1,
                            imm: value,
                        }),

                        Token::Register { num } => inst.push(Instruction::AndRR {
                            d: dest,
                            s1,
                            s2: num,
                        }),
                        t => {
                            return Err(ParseError::UnexpectedToken(format!(
                                "Expected Register or Number, found: {:?}",
                                t
                            )));
                        }
                    }
                    addr += 1;
                }
                Token::Br { n, p, z } => {
                    let label = self.expect_label()?;
                    inst.push(Instruction::Br { n, z, p, label });
                    addr += 1;
                }
                Token::Jmp => {
                    let base = self.expect_register()?;
                    inst.push(Instruction::Jmp { base });
                    addr += 1;
                }
                Token::Jsr => {
                    let label = self.expect_label()?;
                    inst.push(Instruction::Jsr { label });
                    addr += 1;
                }
                Token::Jsrr => {
                    let base = self.expect_register()?;
                    inst.push(Instruction::Jsrr { base });
                    addr += 1;
                }
                Token::Ld => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let label = self.expect_label()?;
                    inst.push(Instruction::Ld { d: dest, label });
                    addr += 1;
                }
                Token::Ldi => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let label = self.expect_label()?;
                    inst.push(Instruction::Ldi { d: dest, label });
                    addr += 1;
                }
                Token::Ldr => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let base = self.expect_register()?;
                    self.expect_comma()?;
                    let imm = self.expect_number()?;
                    inst.push(Instruction::Ldr { d: dest, base, imm });
                    addr += 1;
                }
                Token::Lea => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let label = self.expect_label()?;
                    inst.push(Instruction::Lea { d: dest, label });
                    addr += 1;
                }
                Token::Not => {
                    let dest = self.expect_register()?;
                    self.expect_comma()?;
                    let s1 = self.expect_register()?;
                    inst.push(Instruction::Not { d: dest, s1 });
                    addr += 1;
                }
                Token::Ret => {
                    inst.push(Instruction::Ret);
                    addr += 1;
                }
                Token::Rti => {
                    inst.push(Instruction::Rti);
                    addr += 1;
                }
                Token::St => {
                    let s1 = self.expect_register()?;
                    self.expect_comma()?;
                    let label = self.expect_label()?;
                    inst.push(Instruction::St { s1, label });
                    addr += 1;
                }
                Token::Sti => {
                    let s1 = self.expect_register()?;
                    self.expect_comma()?;
                    let label = self.expect_label()?;
                    inst.push(Instruction::Sti { s1, label });
                    addr += 1;
                }
                Token::Str => {
                    let s1 = self.expect_register()?;
                    self.expect_comma()?;
                    let base = self.expect_register()?;
                    self.expect_comma()?;
                    let imm = self.expect_number()?;
                    inst.push(Instruction::Str { s1, base, imm });
                    addr += 1;
                }
                Token::Trap => {
                    let trapvect = self.expect_number()?;
                    inst.push(Instruction::Trap { trapvect });
                    addr += 1;
                }
                t => {
                    return Err(ParseError::UnexpectedToken(format!(
                        "Unexpected Token: {:?}",
                        t
                    )));
                }
            }
        }
        Ok((symbol_map, inst))
    }
}

pub enum AssemblerError {
    DuplicateLabel(String),
    MissingLabel(String),
    InvalidInstruction(String),
    ParseNumberError,
}

fn assemble(symbol_map: SymbolMap, instructions: Vec<Instruction>) -> Vec<u16> {
    let mut out: Vec<u16> = Vec::new();
    let mut curr_addr: u16 = 0;
    for (i, inst) in instructions.iter().enumerate() {
        match inst {
            Instruction::Orig(addr) => {
                curr_addr = *addr as u16;
                out.push((*addr as u16).to_be());
            }
            Instruction::End => {
                if i != instructions.len() - 1 {
                    panic!(".END found before the end");
                }
            }
            Instruction::Fill(val) => {
                curr_addr += 1;
                out.push((*val as u16).to_be());
            }
            Instruction::Blkw(amount) => {
                curr_addr += *amount as u16;
                out.extend(std::iter::repeat_n(0u16, *amount as usize));
            }
            Instruction::Stringz(string) => {
                curr_addr += string.len() as u16 + 1;
                for c in string.chars() {
                    out.push((c as u16).to_be());
                }
                out.push(0);
            }
            Instruction::AddRR { d, s1, s2 } => {
                curr_addr += 1;
                let (d, s1, s2) = (*d as u16, *s1 as u16, *s2 as u16);
                let word: u16 = (0b0001 << 12) | d << 9 | s1 << 6 | s2;
                out.push(word.to_be());
            }
            Instruction::AddRI { d, s1, imm } => {
                curr_addr += 1;
                let (d, s1, imm) = (*d as u16, *s1 as u16, *imm as u16);
                let imm5 = imm & 0x1F;
                let word: u16 = (0b0001 << 12) | d << 9 | s1 << 6 | 1 << 5 | imm5;
                out.push(word.to_be());
            }
            Instruction::AndRR { d, s1, s2 } => {
                curr_addr += 1;
                let (d, s1, s2) = (*d as u16, *s1 as u16, *s2 as u16);
                let word: u16 = (0b0101 << 12) | d << 9 | s1 << 6 | s2;
                out.push(word.to_be());
            }
            Instruction::AndRI { d, s1, imm } => {
                curr_addr += 1;
                let (d, s1, imm) = (*d as u16, *s1 as u16, *imm as u16);
                let imm5 = imm & 0x1F;
                let word: u16 = (0b0101 << 12) | d << 9 | s1 << 6 | 1 << 5 | imm5;
                out.push(word.to_be());
            }
            Instruction::Not { d, s1 } => {
                curr_addr += 1;
                let (d, s1) = (*d as u16, *s1 as u16);
                let word: u16 = (0b1001 << 12) | d << 9 | s1 << 6 | 0x3f;
                out.push(word.to_be());
            }
            Instruction::Ld { d, label } => {
                curr_addr += 1;
                let d = *d as u16;
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-256..=255).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset9 = (offset & 0x1FF) as u16;
                let word: u16 = (0b0010 << 12) | d << 9 | offset9;
                out.push(word.to_be());
            }
            Instruction::Ldi { d, label } => {
                curr_addr += 1;
                let d = *d as u16;
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-256..=255).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset9 = (offset & 0x1FF) as u16;
                let word: u16 = (0b1010 << 12) | d << 9 | offset9;
                out.push(word.to_be());
            }
            Instruction::Ldr { d, base, imm } => {
                curr_addr += 1;
                let (d, base, imm) = (*d as u16, *base as u16, *imm as u16);
                let offset6 = imm & 0x3F;
                let word: u16 = (0b0110 << 12) | d << 9 | base << 6 | offset6;
                out.push(word.to_be());
            }
            Instruction::Lea { d, label } => {
                curr_addr += 1;
                let d = *d as u16;
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-256..=255).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset9 = (offset & 0x1FF) as u16;
                let word: u16 = (0b1110 << 12) | d << 9 | offset9;
                out.push(word.to_be());
            }
            Instruction::St { s1, label } => {
                curr_addr += 1;
                let s1 = *s1 as u16;
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-256..=255).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset9 = (offset & 0x1FF) as u16;
                let word: u16 = (0b0011 << 12) | s1 << 9 | offset9;
                out.push(word.to_be());
            }
            Instruction::Sti { s1, label } => {
                curr_addr += 1;
                let s1 = *s1 as u16;
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-256..=255).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset9 = (offset & 0x1FF) as u16;
                let word: u16 = (0b1011 << 12) | s1 << 9 | offset9;
                out.push(word.to_be());
            }
            Instruction::Str { s1, base, imm } => {
                curr_addr += 1;
                let (s1, base, imm) = (*s1 as u16, *base as u16, *imm as u16);
                let offset6 = imm & 0x3F;
                let word: u16 = (0b0111 << 12) | s1 << 9 | base << 6 | offset6;
                out.push(word.to_be());
            }
            Instruction::Br { n, z, p, label } => {
                curr_addr += 1;
                let n = if *n { 1u16 } else { 0u16 };
                let z = if *z { 1u16 } else { 0u16 };
                let p = if *p { 1u16 } else { 0u16 };
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-256..=255).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset9 = (offset & 0x1FF) as u16;
                let word: u16 = 0b0000 << 12 | n << 11 | z << 10 | p << 9 | offset9;
                out.push(word.to_be());
            }
            Instruction::Jmp { base } => {
                curr_addr += 1;
                let base = *base as u16;
                let word: u16 = 0b1100 << 12 | base << 6;
                out.push(word.to_be());
            }
            Instruction::Jsr { label } => {
                curr_addr += 1;
                let label_addr = symbol_map
                    .get(label)
                    .copied()
                    .unwrap_or_else(|| panic!("Unknown label: {}", label));
                let offset = label_addr.wrapping_sub(curr_addr as i16);
                if !(-1024..=1023).contains(&offset) {
                    panic!("Label offset out of range");
                }
                let offset11 = (offset & 0x7FF) as u16;
                let word: u16 = 0b01001 << 11 | offset11;
                out.push(word.to_be());
            }
            Instruction::Jsrr { base } => {
                curr_addr += 1;
                let base = *base as u16;
                let word: u16 = 0b0100 << 12 | base << 6;
                out.push(word.to_be());
            }
            Instruction::Ret => {
                curr_addr += 1;
                let word: u16 = 0b1100_0001_1100_0000;
                out.push(word.to_be());
            }
            Instruction::Rti => {
                curr_addr += 1;
                let word: u16 = 1 << 15;
                out.push(word.to_be());
            }
            Instruction::Trap { trapvect } => {
                curr_addr += 1;
                let trapvect = *trapvect as u16;
                if !(0..=255).contains(&trapvect) {
                    panic!("Trap vector out of bounds!");
                }
                let addr8 = trapvect & 0xFF;
                let word: u16 = 0b1111 << 12 | addr8;
                out.push(word.to_be());
            }
        }
    }
    out
}

fn write_obj(path: &str, binary: Vec<u16>) -> std::io::Result<()> {
    let bytes: Vec<u8> = binary
        .iter()
        .flat_map(|word| {
            let [b1, b2] = word.to_ne_bytes();
            [b1, b2]
        })
        .collect();
    std::fs::write(path, bytes)
}

fn main() -> Result<(), ParseError> {
    let body = std::fs::read_to_string("in.asm").unwrap();
    let tokens = match Lexer::new(&body).tokenize() {
        Ok(tokens) => tokens,
        Err(e) => panic!("{:?}", e),
    };
    let (symbol_map, instructions) = Parser::new(tokens).parse()?;
    let binary = assemble(symbol_map, instructions);
    match write_obj("out.obj", binary) {
        Ok(_) => (),
        Err(e) => panic!("Error writing file: {:?}", e),
    };
    Ok(())
}

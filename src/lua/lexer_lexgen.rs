use super::error::LexerError as LexerError_;
use super::lexer_luster as luster;
use super::token::Token;

use lexgen::lexer;

use std::mem::replace;
use std::convert::TryFrom;

#[derive(Debug, Default, Clone)]
pub struct LexerState {
    /// Number of opening `=`s seen when parsing a long string
    long_string_opening_eqs: usize,
    /// Number of closing `=`s seen when parsing a long string
    long_string_closing_eqs: usize,
    /// When parsing a short string, whether it's started with a double or single quote
    short_string_delim: Quote,
    /// Buffer for strings
    string_buf: Vec<u8>,
    /// When parsing a long string, whether we're inside a comment or not. When inside a comment we
    /// don't return a token. Otherwise we return a string.
    in_comment: bool,
    /// Unicode codepoint being parsed.
    unicode_codepoint: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Quote {
    Single,
    Double,
}

impl Default for Quote {
    fn default() -> Self {
        // arbitrary
        Quote::Single
    }
}

lexer! {
    pub Lexer(LexerState) -> Token<Vec<u8>>;

    type Error = LexerError_;

    let whitespace = [' ' '\t' '\n'] | "\r\n";

    // > Names (also called identifiers) in Lua can be any string of letters, digits, and
    // > underscores, not beginning with a digit. This coincides with the definition of names in
    // > most languages. (The definition of letter depends on the current locale: any character
    // > considered alphabetic by the current locale can be used in an identifier.)
    let var_init = ['a'-'z' 'A'-'Z' '_'];
    let var_subseq = $var_init | ['0'-'9'];

    let digit = ['0'-'9'];
    let hex_digit = ['a'-'f' 'A'-'F' '0'-'9'];

    rule Init {
        $whitespace,

        "+" = Token::Add,
        "-" = Token::Minus,
        "*" = Token::Mul,
        "/" = Token::Div,
        "//" = Token::IDiv,
        "%" = Token::Mod,
        "^" = Token::Pow,
        "#" = Token::Len,
        "==" = Token::Equal,
        "~=" = Token::NotEqual,
        "<=" = Token::LessEqual,
        ">=" = Token::GreaterEqual,
        "<" = Token::LessThan,
        ">" = Token::GreaterThan,
        "=" = Token::Assign,
        "(" = Token::LeftParen,
        ")" = Token::RightParen,
        "{" = Token::LeftBrace,
        "}" = Token::RightBrace,
        "]" = Token::RightBracket,
        ";" = Token::SemiColon,
        ":" = Token::Colon,
        "," = Token::Comma,
        "." = Token::Dot,
        ".." = Token::Concat,
        "..." = Token::Dots,
        "&" = Token::BitAnd,
        "|" = Token::BitOr,
        "~" = Token::BitNotXor,
        ">>" = Token::ShiftRight,
        "<<" = Token::ShiftLeft,
        "::" = Token::DoubleColon,
        "and" = Token::And,
        "break" = Token::Break,
        "do" = Token::Do,
        "else" = Token::Else,
        "elseif" = Token::ElseIf,
        "end" = Token::End,
        "false" = Token::False,
        "for" = Token::For,
        "function" = Token::Function,
        "if" = Token::If,
        "in" = Token::In,
        "local" = Token::Local,
        "nil" = Token::Nil,
        "not" = Token::Not,
        "or" = Token::Or,
        "repeat" = Token::Repeat,
        "return" = Token::Return,
        "then" = Token::Then,
        "true" = Token::True,
        "until" = Token::Until,
        "while" = Token::While,
        "goto" = Token::Goto,

        '"' => |mut lexer| {
            lexer.state().short_string_delim = Quote::Double;
            lexer.state().string_buf.clear();
            lexer.switch(LexerRule::String)
        },

        '\'' => |mut lexer| {
            lexer.state().short_string_delim = Quote::Single;
            lexer.state().string_buf.clear();
            lexer.switch(LexerRule::String)
        },

        "[" => |mut lexer| {
            match lexer.peek() {
                Some('[') | Some('=') => {
                    lexer.state().long_string_opening_eqs = 0;
                    lexer.state().in_comment = false;
                    lexer.switch(LexerRule::LongStringBracketLeft)
                }
                _ => lexer.return_(Token::LeftBracket),
            }
        },

        "--" => |lexer| {
            lexer.switch(LexerRule::EnterComment)
        },

        $var_init $var_subseq* => |lexer| {
            let match_ = lexer.match_();
            lexer.return_(Token::Name(match_.as_bytes().to_owned()))
        },

        $digit+ '.'? $digit* (('e' | 'E') ('+'|'-')? $digit+)? =? |lexer| {
            let match_ = lexer.match_();
            lexer.return_(read_numeral(match_))
        },

        '.' $digit+ (('e' | 'E') ('+'|'-')? $digit+)? =? |lexer| {
            let match_ = lexer.match_();
            lexer.return_(read_numeral(match_))
        },

        '0' ('x'|'X') $hex_digit? '.'? $hex_digit* (('p' | 'P') ('+'|'-')? $hex_digit+)? =? |lexer| {
            let match_ = lexer.match_();
            lexer.return_(read_numeral(match_))
        },
    }

    rule LongStringBracketLeft {
        '=' =>
            |mut lexer| {
                lexer.state().long_string_opening_eqs += 1;
                lexer.continue_()
            },

        '[' =>
            |lexer|
                lexer.switch(LexerRule::LongString),
    }

    rule LongString {
        ']' =>
            |mut lexer| {
                lexer.state().long_string_closing_eqs = 0;
                lexer.switch(LexerRule::LongStringBracketRight)
            },

        _ =>
            |lexer|
                lexer.continue_(),
    }

    rule LongStringBracketRight {
        '=' =>
            |mut lexer| {
                lexer.state().long_string_closing_eqs += 1;
                lexer.continue_()
            },

        ']' =>
            |mut lexer| {
                let state = lexer.state();
                let in_comment = state.in_comment;
                let left_eqs = state.long_string_opening_eqs;
                let right_eqs = state.long_string_closing_eqs;
                if left_eqs == right_eqs {
                    if in_comment {
                        lexer.switch(LexerRule::Init)
                    } else {
                        let match_ = &lexer.match_[left_eqs + 2..lexer.match_.len() - right_eqs - 2];
                        lexer.switch_and_return(LexerRule::Init, Token::String(match_.as_bytes().to_owned()))
                    }
                } else {
                    lexer.state().long_string_closing_eqs = 0;
                    lexer.continue_()
                }
            },

        _ =>
            |lexer|
                lexer.switch(LexerRule::LongString),
    }

    rule String {
        '"' => |mut lexer| {
            if lexer.state().short_string_delim == Quote::Double {
                let str = replace(&mut lexer.state().string_buf, Vec::new());
                lexer.switch_and_return(LexerRule::Init, Token::String(str))
            } else {
                lexer.state().string_buf.push(b'"');
                lexer.continue_()
            }
        },

        "'" => |mut lexer| {
            if lexer.state().short_string_delim == Quote::Single {
                let str = replace(&mut lexer.state().string_buf, Vec::new());
                lexer.switch_and_return(LexerRule::Init, Token::String(str))
            } else {
                lexer.state().string_buf.push(b'\'');
                lexer.continue_()
            }
        },

        "\\a" => |mut lexer| {
            lexer.state().string_buf.push(0x7);
            lexer.continue_()
        },

        "\\b" => |mut lexer| {
            lexer.state().string_buf.push(0x8);
            lexer.continue_()
        },

        "\\f" => |mut lexer| {
            lexer.state().string_buf.push(0xc);
            lexer.continue_()
        },

        "\\n" => |mut lexer| {
            lexer.state().string_buf.push(b'\n');
            lexer.continue_()
        },

        "\\r" => |mut lexer| {
            lexer.state().string_buf.push(b'\r');
            lexer.continue_()
        },

        "\\t" => |mut lexer| {
            lexer.state().string_buf.push(b'\t');
            lexer.continue_()
        },

        "\\v" => |mut lexer| {
            lexer.state().string_buf.push(0xb);
            lexer.continue_()
        },

        "\\\\" => |mut lexer| {
            lexer.state().string_buf.push(b'\\');
            lexer.continue_()
        },

        "\\\"" => |mut lexer| {
            lexer.state().string_buf.push(b'"');
            lexer.continue_()
        },

        "\\'" => |mut lexer| {
            lexer.state().string_buf.push(b'\'');
            lexer.continue_()
        },

        "\\\n" => |mut lexer| {
            lexer.state().string_buf.push(b'\n');
            lexer.continue_()
        },

        // TODO: Better way to match 1-3 digits?
        '\\' $digit => |mut lexer| {
            let match_ = lexer.match_();
            let bytes = match_.as_bytes();
            let digit = bytes[bytes.len() - 1] - b'0';
            lexer.state().string_buf.push(digit);
            lexer.continue_()
        },

        '\\' $digit $digit => |mut lexer| {
            let match_ = lexer.match_();
            let bytes = match_.as_bytes();
            let digit1 = bytes[bytes.len() - 2] - b'0';
            let digit2 = bytes[bytes.len() - 1] - b'0';
            lexer.state().string_buf.push(digit1 * 10 + digit2);
            lexer.continue_()
        },

        '\\' $digit $digit $digit => |mut lexer| {
            let match_ = lexer.match_();
            let bytes = match_.as_bytes();
            let digit1 = bytes[bytes.len() - 3] - b'0';
            let digit2 = bytes[bytes.len() - 2] - b'0';
            let digit3 = bytes[bytes.len() - 1] - b'0';
            lexer.state().string_buf.push(
                digit1 * 100 + digit2 * 10 + digit3
            );
            lexer.continue_()
        },

        "\\x" $hex_digit $hex_digit => |mut lexer| {
            let match_ = lexer.match_();
            let bytes = match_.as_bytes();
            // println!("match_={:?}", match_);
            use super::lexer_luster::from_hex_digit;
            let digit1 = from_hex_digit(bytes[bytes.len() - 2]).unwrap();
            let digit2 = from_hex_digit(bytes[bytes.len() - 1]).unwrap();
            // println!("digit1={}, digit2={}", digit1, digit2);
            lexer.state().string_buf.push(
                digit1 * 16 + digit2
            );
            lexer.continue_()
        },

        // TODO: This is implemented as a separate rule to as otherwise it's difficult to get the
        // match for the hex characters only (instead of the entire match that includes "\x{" and
        // stuff before it). We should allow binding regexes inside patterns.
        "\\u{" => |mut lexer| {
            lexer.state().unicode_codepoint = 0;
            lexer.switch(LexerRule::UnicodeCodepoint)
        },

        "\\z" $whitespace*,

        _ => |mut lexer| {
            let char = lexer.match_().chars().next_back().unwrap();
            let state = lexer.state();
            let char_utf8_len = char.len_utf8();
            let cursor = state.string_buf.len();
            state.string_buf.reserve(char_utf8_len);
            for _ in 0 .. char_utf8_len {
                state.string_buf.push(0);
            }
            char.encode_utf8(&mut state.string_buf[cursor..]);
            lexer.continue_()
        },
    }

    rule UnicodeCodepoint {
        $hex_digit => |mut lexer| {
            let c = lexer.match_().chars().next_back().unwrap();
            let digit = if c >= '0' && c <= '9' {
                c as u32 - '0' as u32
            } else if c >= 'a' && c <= 'f' {
                c as u32 - 'a' as u32 + 10
            } else {
                c as u32 - 'A' as u32 + 10
            };

            let state = lexer.state();
            state.unicode_codepoint *= 16;
            state.unicode_codepoint += digit;

            lexer.continue_()
        },

        '}' => |mut lexer| {
            let state = lexer.state();
            let char = char::try_from(state.unicode_codepoint).unwrap();
            let char_utf8_len = char.len_utf8();
            let cursor = state.string_buf.len();
            state.string_buf.reserve(char_utf8_len);
            for _ in 0 .. char_utf8_len {
                state.string_buf.push(0);
            }
            char.encode_utf8(&mut state.string_buf[cursor..]);
            lexer.switch(LexerRule::String)
        },
    }

    rule EnterComment {
        '[' => |mut lexer| {
            match lexer.peek() {
                Some('[') | Some('=') => {
                    lexer.state().long_string_opening_eqs = 0;
                    lexer.state().in_comment = true;
                    lexer.switch(LexerRule::LongStringBracketLeft)
                }
                _ =>
                    lexer.switch(LexerRule::Comment),
            }
        },

        _ => |lexer|
            lexer.switch(LexerRule::Comment),
    }

    rule Comment {
        '\n' => |lexer|
            lexer.switch(LexerRule::Init),

        _ => |lexer|
            lexer.continue_(),
    }
}

fn read_numeral<S>(s: &str) -> Result<Token<S>, LexerError_> {
    // println!("read_numeral({:?})", s);
    luster::Lexer::new(s.as_bytes(), |_| panic!()).read_numeral()
}

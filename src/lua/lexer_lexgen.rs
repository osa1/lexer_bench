use super::error::LexerError as LexerError_;
use super::lexer_luster::{read_float, read_hex_float, read_hex_integer, read_integer};
use super::token::Token;

use lexgen::lexer;

#[derive(Debug, Default, Clone)]
struct LexerState {
    /// Number of opening `=`s seen when parsing a long string
    long_string_opening_eqs: usize,
    /// Number of closing `=`s seen when parsing a long string
    long_string_closing_eqs: usize,
    /// When parsing a short string, whether it's started with a double or single quote
    short_string_delim: Quote,
    /// Buffer for strings
    string_buf: String,
    /// When parsing a long string, whether we're inside a comment or not. When inside a comment we
    /// don't return a token. Otherwise we return a string.
    in_comment: bool,
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
    Lexer(LexerState) -> Token<&'input str>;

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
            // lexer.switch(LexerRule::String)
            todo!()
        },

        '\'' => |mut lexer| {
            lexer.state().short_string_delim = Quote::Single;
            lexer.state().string_buf.clear();
            // lexer.switch(LexerRule::String)
            todo!()
        },

        "[" => |mut lexer| {
            match lexer.peek() {
                Some('[') | Some('=') => {
                    lexer.state().long_string_opening_eqs = 0;
                    lexer.state().in_comment = false;
                    // lexer.switch(LexerRule::LongStringBracketLeft)
                    todo!()
                }
                _ => lexer.return_(Token::LeftBracket),
            }
        },

        "--" => |lexer| {
            lexer.switch(LexerRule::EnterComment)
        },

        $var_init $var_subseq* => |lexer| {
            let match_ = lexer.match_();
            lexer.return_(Token::Name(match_))
        },

        $digit+ ('.'? $digit+ (('e' | 'E') ('+'|'-')? $digit+)?)? =? |lexer| {
            let match_ = lexer.match_();
            lexer.return_(read_numeral(match_))
        },

        "0x" $hex_digit+ =? |lexer| {
            let match_ = lexer.match_();
            lexer.return_(read_numeral(match_))
        },
    }

/*
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
                        lexer.switch_and_return(LexerRule::Init, Token::String(StringToken::Raw(match_)))
                    }
                } else {
                    lexer.switch(LexerRule::String)
                }
            },

        _ =>
            |lexer|
                lexer.switch(LexerRule::String),
    }

    rule String {
        '"' => |mut lexer| {
            if lexer.state().short_string_delim == Quote::Double {
                let str = lexer.state().string_buf.clone();
                lexer.switch_and_return(LexerRule::Init, Token::String(StringToken::Interpreted(str)))
            } else {
                lexer.state().string_buf.push('"');
                lexer.continue_()
            }
        },

        "'" => |mut lexer| {
            if lexer.state().short_string_delim == Quote::Single {
                let str = lexer.state().string_buf.clone();
                lexer.switch_and_return(LexerRule::Init, Token::String(StringToken::Interpreted(str)))
            } else {
                lexer.state().string_buf.push('\'');
                lexer.continue_()
            }
        },

        "\\a" => |mut lexer| {
            lexer.state().string_buf.push('\u{7}');
            lexer.continue_()
        },

        "\\b" => |mut lexer| {
            lexer.state().string_buf.push('\u{8}');
            lexer.continue_()
        },

        "\\f" => |mut lexer| {
            lexer.state().string_buf.push('\u{c}');
            lexer.continue_()
        },

        "\\n" => |mut lexer| {
            lexer.state().string_buf.push('\n');
            lexer.continue_()
        },

        "\\r" => |mut lexer| {
            lexer.state().string_buf.push('\r');
            lexer.continue_()
        },

        "\\t" => |mut lexer| {
            lexer.state().string_buf.push('\t');
            lexer.continue_()
        },

        "\\v" => |mut lexer| {
            lexer.state().string_buf.push('\u{b}');
            lexer.continue_()
        },

        "\\\\" => |mut lexer| {
            lexer.state().string_buf.push('\\');
            lexer.continue_()
        },

        "\\\"" => |mut lexer| {
            lexer.state().string_buf.push('"');
            lexer.continue_()
        },

        "\\'" => |mut lexer| {
            lexer.state().string_buf.push('\'');
            lexer.continue_()
        },

        "\\\n" => |mut lexer| {
            lexer.state().string_buf.push('\n');
            lexer.continue_()
        },

        _ => |mut lexer| {
            let char = lexer.match_().chars().next_back().unwrap();
            lexer.state().string_buf.push(char);
            lexer.continue_()
        },
    }
*/

    rule EnterComment {
        '[' => |mut lexer| {
            match lexer.peek() {
                Some('[') | Some('=') => {
                    lexer.state().long_string_opening_eqs = 0;
                    lexer.state().in_comment = true;
                    // lexer.switch(LexerRule::LongStringBracketLeft)
                    todo!()
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
    let mut chars = s.chars().peekable();

    let p1 = chars.next().unwrap();
    assert!(p1 == '.' || p1.is_ascii_digit());

    let mut string_buffer = String::new();

    let p2 = chars.peek().copied();
    let is_hex = p1 == '0' && (p2 == Some('x') || p2 == Some('X'));
    if is_hex {
        string_buffer.push(p1);
        string_buffer.push(p2.unwrap());
        chars.next();
    }

    let mut has_radix = false;
    while let Some(c) = chars.peek().copied() {
        if c == '.' && !has_radix {
            string_buffer.push('.');
            has_radix = true;
            chars.next();
        } else if (!is_hex && c.is_ascii_digit()) || (is_hex && c.is_ascii_hexdigit()) {
            string_buffer.push(c);
            chars.next();
        } else {
            break;
        }
    }

    let mut has_exp = false;
    if let Some(exp_begin) = chars.peek().copied() {
        if (is_hex && (exp_begin == 'p' || exp_begin == 'P'))
            || (!is_hex && (exp_begin == 'e' || exp_begin == 'E'))
        {
            string_buffer.push(exp_begin);
            has_exp = true;
            chars.next();

            if let Some(sign) = chars.peek().copied() {
                if sign == '+' || sign == '-' {
                    string_buffer.push(sign);
                    chars.next();
                }
            }

            while let Some(c) = chars.peek().copied() {
                if c.is_ascii_digit() {
                    string_buffer.push(c);
                    chars.next();
                } else {
                    break;
                }
            }
        }
    }

    if !has_exp && !has_radix {
        if is_hex {
            if let Some(i) = read_hex_integer(string_buffer.as_bytes()) {
                return Ok(Token::Integer(i));
            }
        }
        if let Some(i) = read_integer(string_buffer.as_bytes()) {
            return Ok(Token::Integer(i));
        }
    }

    Ok(Token::Float(
        if is_hex {
            read_hex_float(string_buffer.as_bytes())
        } else {
            read_float(string_buffer.as_bytes())
        }
        .ok_or(LexerError_::BadNumber)?,
    ))
}

use std::{fmt, io};

#[derive(Debug)]
pub enum LexerError {
    UnfinishedShortString(u8),
    UnexpectedCharacter(u8),
    HexDigitExpected,
    EscapeUnicodeStart,
    EscapeUnicodeEnd,
    EscapeUnicodeInvalid,
    EscapeDecimalTooLarge,
    InvalidEscape,
    InvalidLongStringDelimiter,
    UnfinishedLongString,
    BadNumber,
    IOError(io::Error),
}

impl fmt::Display for LexerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn print_char(c: u8) -> char {
            char::from_u32(c as u32).unwrap_or(char::REPLACEMENT_CHARACTER)
        }

        match self {
            LexerError::UnfinishedShortString(c) => write!(
                f,
                "short string not finished, expected matching {}",
                print_char(*c)
            ),
            LexerError::UnexpectedCharacter(c) => {
                write!(f, "unexpected character: '{}'", print_char(*c))
            }
            LexerError::HexDigitExpected => write!(f, "hexadecimal digit expected"),
            LexerError::EscapeUnicodeStart => write!(f, "missing '{{' in \\u{{xxxx}} escape"),
            LexerError::EscapeUnicodeEnd => write!(f, "missing '}}' in \\u{{xxxx}} escape"),
            LexerError::EscapeUnicodeInvalid => {
                write!(f, "invalid unicode value in \\u{{xxxx}} escape")
            }
            LexerError::EscapeDecimalTooLarge => write!(f, "\\ddd escape out of 0-255 range"),
            LexerError::InvalidEscape => write!(f, "invalid escape sequence"),
            LexerError::InvalidLongStringDelimiter => write!(f, "invalid long string delimiter"),
            LexerError::UnfinishedLongString => write!(f, "unfinished long string"),
            LexerError::BadNumber => write!(f, "malformed number"),
            LexerError::IOError(err) => write!(f, "IO Error: {}", err),
        }
    }
}

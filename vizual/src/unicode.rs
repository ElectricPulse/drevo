use std::convert::TryFrom;
use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    InvalidEndingSequence { bytes: Vec<u8> },
    InvalidSequence,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidSequence => write!(f, "invalid utf8 sequence"),
            Error::InvalidEndingSequence { bytes: _ } => {
                write!(f, "invalid utf8 sequence at the end of stream")
            }
        }
    }
}

// Internal error for fetching a continuation byte
#[derive(Debug, Clone)]
enum Next_err {
    Eof,
    Invalid,
}

fn next_byte<I: Iterator<Item = u8>>(iter: &mut I) -> Result<u32, Next_err> {
    match iter.next() {
        Some(c) => {
            if c & 0xC0 == 0x80 {
                Ok((c & 0x3F) as u32)
            } else {
                Err(Next_err::Invalid)
            }
        }
        None => Err(Next_err::Eof),
    }
}

fn raw_decode_from<I: Iterator<Item = u8>>(a: u32, iter: &mut I) -> Result<u32, Error> {
    let mut bytes = vec![a as u8];

    let need = |iter: &mut I, bytes: &mut Vec<u8>| -> Result<u32, Error> {
        let byte = next_byte(iter).map_err(|e| match e {
            Next_err::Eof => Error::InvalidEndingSequence {
                bytes: bytes.to_vec(),
            },
            Next_err::Invalid => Error::InvalidSequence,
        })?;

        bytes.push(byte as u8);

        Ok(byte)
    };

    if a & 0x80 == 0x00 {
        Ok(a)
    } else if a & 0xE0 == 0xC0 {
        let b = need(iter, &mut bytes)?;
        Ok((a & 0x1F) << 6 | b)
    } else if a & 0xF0 == 0xE0 {
        let b = need(iter, &mut bytes)?;
        let c = need(iter, &mut bytes)?;
        Ok((a & 0x0F) << 12 | b << 6 | c)
    } else if a & 0xF8 == 0xF0 {
        let b = need(iter, &mut bytes)?;
        let c = need(iter, &mut bytes)?;
        let d = need(iter, &mut bytes)?;
        Ok((a & 0x07) << 18 | b << 12 | c << 6 | d)
    } else {
        Err(Error::InvalidSequence)
    }
}

fn decode_from<I: Iterator<Item = u8>>(a: u32, iter: &mut I) -> Result<char, Error> {
    match char::try_from(raw_decode_from(a, iter)?) {
        Ok(c) => Ok(c),
        Err(_) => Err(Error::InvalidSequence),
    }
}

pub fn decode<I: Iterator<Item = u8>>(iter: &mut I) -> Option<Result<char, Error>> {
    iter.next().map(|a| decode_from(a as u32, iter))
}

pub struct Decoder<R: Iterator<Item = u8>> {
    bytes: R,
}

impl<R: Iterator<Item = u8>> Decoder<R> {
    pub fn new(source: R) -> Decoder<R> {
        Decoder { bytes: source }
    }
}

impl<R: Iterator<Item = u8>> Iterator for Decoder<R> {
    type Item = Result<char, Error>;
    fn next(&mut self) -> Option<Result<char, Error>> {
        decode(&mut self.bytes)
    }
}

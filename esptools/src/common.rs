// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

use std::{
    cmp::min,
    io::{self, BorrowedBuf, Read, Seek, SeekFrom},
    mem::{MaybeUninit, size_of}, ffi::{CStr, CString},
};

use thiserror::Error;
pub trait ConstantSizedRecord {
    const SIZE: usize;
}

pub trait RawRecord {
	type Raw;
}
impl<R: RawRecord> ConstantSizedRecord for R {
    const SIZE: usize = size_of::<R::Raw>();
}

pub struct ReadSpan {
    pub offset: u64,
    pub size: usize,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("Invalid Type Tag")]
    InvalidTag,
    #[error("Bogus size in file data")]
    BogusSize,
    #[error("Parsing error")]
    ParseError,
    #[error("IO Error")]
    Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[rustc_specialization_trait]
pub trait SeekPredicate: Seek {
    fn unconditionally_seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek(pos)
    }
}

trait ReadExtSkip {
    fn skip_ext(&mut self, n: u64) -> io::Result<()>;
}

pub(crate) trait SkipExt: Read + Seek + Sized {
	fn parse_from_span(&mut self, span: ReadSpan) -> io::Result<Box<[u8]>> {
		self.seek(SeekFrom::Start(span.offset))?;
		self.parse_bytes(span.size)
	}
}

pub(crate) trait ReadExt: Read + Sized {
    fn parse_bytes(&mut self, n: usize) -> io::Result<Box<[u8]>> {
        let mut result = Box::<[u8]>::new_uninit_slice(n);
        let mut bbuf: BorrowedBuf = result.as_mut().into();
        self.read_buf_exact(bbuf.unfilled())?;
        Ok(unsafe { result.assume_init() })
    }
    fn skip(&mut self, n: u64) -> io::Result<()> {
        self.skip_ext(n)
	}
	fn eat_tag<const N: usize>(&mut self, tag: &'static [u8; N]) -> Result<[u8; N]> {
		let buf: [u8; N] = self.parse()?;
		if buf == *tag {
			Ok(*tag)
		} else {
			Err(Error::InvalidTag)
		}
	}
	fn parse_bzstring(&mut self) -> Result<CString> {
		let result = self.parse_bstring()?;
		match CStr::from_bytes_with_nul(&result) {
			Ok(s) => Ok(s.into()),
			Err(_) => Err(Error::ParseError)
		}
	}

	fn parse_bstring(&mut self) -> Result<Box<[u8]>> {
		let len  = u8::from_le_bytes(self.parse()?);
		Ok(self.parse_bytes(len as usize)?)
	}

	fn parse<T: ParseCommon>(&mut self) -> Result<T> {
		<T as ParseCommon>::parse(self)
	}
}

impl<R: Read> ReadExt for R {}

impl<R: Read> ReadExtSkip for R {
    default fn skip_ext(&mut self, mut n: u64) -> io::Result<()> {
        println!("Unbuffered");
        let mut buf: [MaybeUninit<u8>; 255] = MaybeUninit::uninit_array();

        let mut bbuf = BorrowedBuf::from(&mut buf[0..n as usize]);
        loop {
            let sz = min(255, n);
            if sz == 0 {
                break;
            }
            self.read_buf_exact(bbuf.unfilled())?;
            bbuf.clear();
            n -= sz;
        }
        Ok(())
    }
}

impl<R: Read + SeekPredicate> ReadExtSkip for R {
    fn skip_ext(&mut self, n: u64) -> io::Result<()> {
        println!("Buffered");
        self.seek(SeekFrom::Current(n as i64))?;
        Ok(())
    }
}

pub(crate) trait ParseCommon: Sized {
	fn parse(input: &mut impl Read) -> Result<Self>;
}

impl<const N: usize> ParseCommon for [u8; N] {
	fn parse(input: &mut impl Read) -> Result<Self> {
		let mut buf: [MaybeUninit<u8>; N] = MaybeUninit::uninit_array();
		let mut bbuf: BorrowedBuf = (&mut buf[..]).into();
		input.read_buf_exact(bbuf.unfilled())?;
		Ok(unsafe { MaybeUninit::array_assume_init(buf) })
	}
}

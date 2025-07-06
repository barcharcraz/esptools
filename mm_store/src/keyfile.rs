// SPDX-FileCopyrightText: Charles Barto
//
// SPDX-License-Identifier: LGPL-3.0-only

// parser for GKeyFile/xdg desktop entry files, probably broken, doesn't do localization stuff right now

use std::{
    cmp::min,
    io::{self, BorrowedBuf, Read, Seek, SeekFrom},
    mem::MaybeUninit, iter::{FusedIterator, FlatMap}, fmt::{Display, Write as FmtWrite}, str::Chars,
};

#[rustc_specialization_trait]
trait SeekPredicate: Seek {
    fn unconditionally_seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek(pos)
    }
}

trait SkipExt: Read + Sized {
    fn skip(&mut self, n: u64) -> io::Result<()>;
}
impl<R: Read> SkipExt for R {
    default fn skip(&mut self, mut n: u64) -> io::Result<()> {
        let mut buf = [const { MaybeUninit::<u8>::uninit() }; 255];
        loop {
            let sz = min(255, n);
            if sz == 0 {
                break;
            }
            self.read_buf_exact(BorrowedBuf::from(&mut buf[..sz as usize]).unfilled())?;
            n -= sz;
        }
        Ok(())
    }
}

impl<R: Read + SeekPredicate> SkipExt for R {
    fn skip(&mut self, n: u64) -> io::Result<()> {
        self.seek(SeekFrom::Current(n as i64))?;
        Ok(())
    }
}

impl SeekPredicate for std::fs::File {}

impl SeekPredicate for std::io::BufReader<std::fs::File> {}

// this is basically the same as the standard library does but we need to do it outselves
// because we can't customize that version
#[derive(Clone, Copy)]
enum EscapeState {
    Done,
    Char(char),
    Backslash(char),
}
#[derive(Clone, Copy)]
struct Escape {
    state: EscapeState,
}
impl Iterator for Escape {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        use EscapeState::*;
        match self.state {
            Done => None,
            Char(c) => {
                self.state = Done;
                Some(c)
            }
            Backslash(c) => {
                self.state = Char(c);
                Some('\\')
            }
        }
    }

    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        use EscapeState::*;
        match self.state {
            Done => None,
            Char(c) => {
                self.state = Done;
                if n == 0 {
                    Some(c)
                } else {
                    None
                }
            }
            Backslash(c) if n == 0 => {
                self.state = Char(c);
                Some('\\')
            }
            Backslash(c) if n == 1 => {
                self.state = Done;
                Some(c)
            }
            Backslash(_) => {
                self.state = Done;
                None
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        use EscapeState::*;
        let size = match self.state {
            Done => 0,
            Char(_) => 1,
            Backslash(_) => 2,
        };
        (size, Some(size))
    }
}
impl ExactSizeIterator for Escape {}
impl FusedIterator for Escape {}
impl Display for Escape {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.clone().try_for_each(|c| f.write_char(c))
    }
}

struct EscapeString<'a, F: Clone + Fn(char) -> Escape>(FlatMap<Chars<'a>, Escape, F>);

impl<'a, F: Clone + Fn(char) -> Escape> Display for EscapeString<'a, F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.clone().try_for_each(|c|f.write_char(c))
    }
}
fn escape_char_newline(v: char) -> Escape {
    use EscapeState::*;
    Escape {
        state: match v {
            '\n' => Backslash('n'),
            c => Char(c),
        }
    }
}
fn escape_string_newline<'a>(v: &'a str) -> EscapeString<'a, fn(char) -> Escape> {
    EscapeString(v.chars().flat_map(escape_char_newline))
}
mod error {
    use std::{fmt::Display, io};

    use serde::{de, ser};
    use thiserror::Error;
    #[derive(Debug, Error)]
    pub enum Error {
        #[error("{0}")]
        Custom(String),
        #[error("Data type unsupported")]
        UnsupportedData,
        #[error(transparent)]
        Io(#[from] io::Error),
    }

    impl ser::Error for Error {
        fn custom<T: Display>(msg: T) -> Self {
            Self::Custom(msg.to_string())
        }
    }
    impl de::Error for Error {
        fn custom<T: Display>(msg: T) -> Self {
            Self::Custom(msg.to_string())
        }
    }
}
mod ser {
    use std::{
        fmt::{Display, Write as FmtWrite},
        io::Write,
        iter::{FusedIterator, IntoIterator}, str::{EscapeDebug, EscapeDefault},
    };

    use super::{error::Error, EscapeState, Escape, escape_char_newline, escape_string_newline};
    use base64::Engine;
    use paste::paste;
    use serde::{
        ser::{self, Impossible, SerializeStruct},
        Serialize,
    };
    use serde_ini::ser::UnsupportedType;
    pub struct Serializer<T: Write> {
        output: T,
    }

    macro_rules! serialize_with_format {
        ($($typ:ty),*) => {
            paste! {
                $(
                    fn [<serialize_ $typ>] (self, v: $typ) -> Result<Self::Ok, Self::Error> {
                        write!(self.output, "{}", v)?;
                        Ok(())
                    }
                )*
            }
        };
    }
    
    impl<'a, O: Write> ser::Serializer for &'a mut Serializer<O> {
        type Ok = ();

        type Error = Error;

        type SerializeSeq = Impossible<Self::Ok, Self::Error>;

        type SerializeTuple = Impossible<Self::Ok, Self::Error>;

        type SerializeTupleStruct = Impossible<Self::Ok, Self::Error>;

        type SerializeTupleVariant = Impossible<Self::Ok, Self::Error>;

        type SerializeMap = Impossible<Self::Ok, Self::Error>;

        type SerializeStruct = Self;

        type SerializeStructVariant = Impossible<Self::Ok, Self::Error>;
        serialize_with_format!(bool, i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {

            write!(self.output, "{}", escape_char_newline(v))?;
            Ok(())
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            write!(self.output, "{}", escape_string_newline(v))?;
            Ok(())
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            write!(self.output, "{}", base64::engine::general_purpose::STANDARD_NO_PAD.encode(v))?;
            Ok(())
        }
        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(())
        }

        fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            value.serialize(self)
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(())
        }

        fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok, Self::Error> {
            Ok(())
        }

        fn serialize_unit_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
        ) -> Result<Self::Ok, Self::Error> {
            write!(self.output, "{}", variant)?;
            Ok(())
        }

        fn serialize_newtype_struct<T: ?Sized>(
            self,
            _name: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            value.serialize(self)
        }

        fn serialize_newtype_variant<T: ?Sized>(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: Serialize,
        {
            Err(Error::UnsupportedData)
        }

        fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Err(Error::UnsupportedData)
        }

        fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Err(Error::UnsupportedData)
        }

        fn serialize_tuple_struct(
            self,
            name: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Err(Error::UnsupportedData)
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Err(Error::UnsupportedData)
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Err(Error::UnsupportedData)
        }

        fn serialize_struct(
            self,
            name: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStruct, Self::Error> {
            writeln!(self.output, "[{}]", name)?;
            Ok(self)
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            _variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Err(Error::UnsupportedData)
        }
    }
    impl<'a, O: Write> SerializeStruct for &'a mut Serializer<O> {
        type Ok = ();

        type Error = Error;

        fn serialize_field<T: ?Sized + Serialize>(
            &mut self,
            key: &'static str,
            value: &T,
        ) -> Result<(), Self::Error> {
            write!(self.output, "{}=", key)?;
            value.serialize(&mut **self)?;
            writeln!(self.output, "")?;
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            writeln!(self.output, "")?;
            Ok(())
        }
    }
}
mod de {}

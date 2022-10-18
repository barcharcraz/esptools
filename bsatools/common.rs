use std::ffi::{CStr, CString};
use nom::error::ParseError;
use nom::{InputLength, Parser, ToUsize, InputTake, Finish};
use nom::combinator::{map_res, map_opt, into};
use nom::multi::length_data;
use nom::number::complete::le_u8;
use nom::IResult;
pub struct BzString {
    data: [u8],
}

fn length_data_p1<I, N, E, F>(f: F) -> impl FnMut(I) -> IResult<I, I, E> 
where
    I: Clone + InputLength + InputTake,
    N: ToUsize,
    F: Parser<I, N, E>,
    E: ParseError<I>
{
    length_data(map_opt(f, |i| { i.to_usize().checked_add(1) }))
}
type ErrType<'a> = nom::error::Error<&'a [u8]>;
impl BzString {
    fn parse_to_cstr(input: &[u8]) -> IResult<&[u8], &CStr> {
        map_res(length_data_p1(le_u8), CStr::from_bytes_with_nul)(input)
    }
    fn parse_to_cstring(input: &[u8]) -> IResult<&[u8], CString> {
        into(BzString::parse_to_cstr)(input)
    }
}

impl<'a> TryFrom<&'a BzString> for &'a CStr {
    type Error = ErrType<'a>;

    fn try_from(value: &'a BzString) -> Result<Self, Self::Error> {
        Ok(BzString::parse_to_cstr(&value.data).finish()?.1)
    }
}

impl<'a> TryFrom<&'a BzString> for CString {
    type Error = ErrType<'a>;

    fn try_from(value: &'a BzString) -> Result<Self, Self::Error> {
        Ok(BzString::parse_to_cstring(&value.data).finish()?.1)
    }
    
}

#[test]
fn test_bzstring() {
    let test_data = [5, b'h', b'e', b'l', b'l', b'o', 0];
    let test_bz = unsafe {&*(&test_data as *const [u8] as *const BzString)};
    let test_cstr = TryInto::<&CStr>::try_into(test_bz).unwrap();
    assert_eq!(test_cstr.to_str().unwrap(), "hello");
    let test_cstring: CString = test_bz.try_into().unwrap();
    assert_eq!(test_cstring.to_str().unwrap(), "hello");
}

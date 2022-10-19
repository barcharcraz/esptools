
pub struct BzString {
    data: [u8],
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

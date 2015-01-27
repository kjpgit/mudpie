//! Byte slice manipulation / searching routines, that
//! really should be in the stdlib, in an optimized form.


/// Return position of needle in haystack.
///
/// # Panics
/// Needle must be not empty.
///
pub fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        panic!("memmem: empty needle");
    }
    let mut idx = 0;
    for w in haystack.windows(needle.len()) {
        if w == needle {
            return Some(idx);
        }
        idx += 1;
    }
    return None;
}


/// Split src on a single byte.  
///
/// Note: Wrapper for splitn()
pub fn split_bytes_on(src: &[u8], b: u8, max_splits: usize) -> Vec<&[u8]> {
    let is_match = |&:f: &u8| { (*f == b) };
    let mut ret = Vec::<&[u8]>::new();
    for x in src.splitn(max_splits, is_match) {
        ret.push(x);
    }
    return ret;
}


/// Split src on b"\r\n"
///
/// Note: a final element without a trailing \r\n will be ignored.
pub fn split_bytes_on_crlf(src: &[u8]) -> Vec<&[u8]> {
    let mut start_idx = 0;
    let mut current_idx = 0;
    let mut ret = Vec::<&[u8]>::new();
    for w in src.windows(2) {
        if w == b"\r\n" {
            ret.push(&src[start_idx..current_idx]);
            start_idx = current_idx + 2;
        }
        current_idx += 1;
    }
    return ret;
}


/// Return hexadecimal value of byte, or None
fn to_hexval(byte: u8) -> Option<u8> {
    match byte {
        b'A'...b'F' => Some(byte - b'A' + 10),
        b'a'...b'f' => Some(byte - b'a' + 10),
        b'0'...b'9' => Some(byte - b'0'),
        _ => None
    }
}


/// Return decimal value of byte, or None
fn to_decval(byte: u8) -> Option<u8> {
    match byte {
        b'0'...b'9' => Some(byte - b'0'),
        _ => None
    }
}


/// Decode %XX hex escapes.
pub fn percent_decode(input: &[u8]) -> Vec<u8> {
    let mut i = 0;
    let mut ret = Vec::new();
    loop {
        if i == input.len() {
            return ret;
        }
        if input[i] == b'%' && i+2 < input.len() {
            let l = to_hexval(input[i+1]);
            let r = to_hexval(input[i+2]);
            if l.is_some() && r.is_some() {
                let l = l.unwrap();
                let r = r.unwrap();
                let val = (l << 4) + r;
                // encoded char
                ret.push(val);
                i += 3;
                continue;
            }
        }

        // not encoded
        ret.push(input[i]);
        i += 1;
    }
}


/// Remove leading and trailing spaces (b' ') from input
pub fn strip(input: &[u8]) -> &[u8] {
    return lstrip(rstrip(input));
}


/// Remove leading spaces (b' ') from input, without copying
pub fn lstrip(input: &[u8]) -> &[u8] {
    let mut pos = 0;
    for c in input.iter() {
        if *c != b' ' {
            break;
        }
        pos += 1;
    }
    return &input[pos..];
}


/// Remove trailing spaces (b' ') from input, without copying
pub fn rstrip(input: &[u8]) -> &[u8] {
    let mut ret = input;
    while ret.len() > 0 {
        if ret[ret.len() - 1] != b' ' {
            break;
        }
        ret = &ret[.. ret.len() - 1];
    }
    return ret;
}


/// Parse a number from ascii text 
pub fn parse_u64(input: &[u8]) -> Option<u64> {
    if input.is_empty() {
        return None;
    }
    let mut ret: u64 = 0;
    for c in input.iter() {
        let c_val = to_decval(*c);
        match c_val {
            Some(n) => ret = ret * 10 + n as u64,
            None => return None
        }
    }
    return Some(ret);
}


#[test]
fn test_memmem() {
    let a = b"hello world dude";
    let res = memmem(a, b" wor");
    assert!(res.is_some());
    assert!(res.unwrap() == 5);

    let res = memmem(a, b" work");
    assert!(res.is_none());

    let res = memmem(a, b"hell");
    assert!(res.is_some());
    assert!(res.unwrap() == 0);
}

#[test]
fn test_split_bytes() {
    let a = b"hello world dude";
    let parts = split_bytes_on(&*a, b' ', 10);
    assert!(parts.len() == 3);
    assert!(parts[0] == b"hello");
    assert!(parts[1] == b"world");
    assert!(parts[2] == b"dude");

    let parts = split_bytes_on(&*a, b' ', 1);
    assert!(parts.len() == 2);
    assert!(parts[0] == b"hello");
    assert!(parts[1] == b"world dude");

    let parts = split_bytes_on(b"    ", b' ', 2);
    assert_eq!(parts.len(), 3);
    assert_eq!(parts[0], b"");
    assert_eq!(parts[1], b"");
    assert_eq!(parts[2], b"  ");
}

#[test]
fn test_split_crlf() {
    let a = b"hello world\r\ndude\r\n\r\nlast one\r\n";
    let parts = split_bytes_on_crlf(&*a);
    assert_eq!(parts.len(),  4);
    assert!(parts[0] == b"hello world");
    assert!(parts[1] == b"dude");
    assert!(parts[2] == b"");
    assert!(parts[3] == b"last one");
}

#[test]
fn test_percent_decode() {
    assert!(to_hexval(b'g').is_none());
    assert!(to_hexval(b'Z').is_none());
    assert!(to_hexval(b'f').unwrap() == 15);
    assert!(to_hexval(b'a').unwrap() == 10);
    assert!(to_hexval(b'F').unwrap() == 15);
    assert!(to_hexval(b'A').unwrap() == 10);
    assert!(to_hexval(b'0').unwrap() == 0);
    assert!(to_hexval(b'3').unwrap() == 3);
    assert!(to_hexval(b'9').unwrap() == 9);
    assert_eq!(percent_decode(b"/hi%20there%ff%00"), b"/hi there\xff\x00");
    assert_eq!(percent_decode(b"/%fe%01%"), b"/\xfe\x01%");
    assert_eq!(percent_decode(b"/%fg%zz"), b"/%fg%zz");
    assert_eq!(percent_decode(b"%"), b"%");
    assert_eq!(percent_decode(b"%%"), b"%%");
    assert_eq!(percent_decode(b"%%%"), b"%%%");
}

#[test]
fn test_lstrip() {
    assert_eq!(lstrip(b"  there now "), b"there now ");
    assert_eq!(lstrip(b"here now "), b"here now ");
    assert_eq!(lstrip(b"   "), b"");
    assert_eq!(lstrip(b""), b"");
}


#[test]
fn test_rstrip() {
    assert_eq!(rstrip(b"  there now "), b"  there now");
    assert_eq!(rstrip(b" here now"), b" here now");
    assert_eq!(rstrip(b"   "), b"");
    assert_eq!(rstrip(b""), b"");
}

#[test]
fn test_decval() {
    assert_eq!(to_decval(b'0').unwrap(), 0);
    assert_eq!(parse_u64(b"12345").unwrap(), 12345);

    assert_eq!(parse_u64(b"12345").unwrap(), 12345);
    assert_eq!(parse_u64(b"12345999123").unwrap(), 12345999123);
    assert_eq!(parse_u64(b"1234512345999123").unwrap(), 1234512345999123);

    assert!(parse_u64(b" 12345").is_none());
    assert!(parse_u64(b"12345 ").is_none());
    assert!(parse_u64(b" ").is_none());
    assert!(parse_u64(b"").is_none());
    assert!(parse_u64(b"123a").is_none());
    assert!(parse_u64(b"-123").is_none());
    assert!(parse_u64(b"bcd").is_none());
}

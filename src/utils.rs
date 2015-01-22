/// Needle must be not empty.
/// This really needs to be in stdlib, and optimized
pub fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() == 0 {
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


/// Split src on a single byte
pub fn split_bytes_on(src: &[u8], b: u8, max_splits: usize) -> Vec<&[u8]> {
    let is_match = |&:f: &u8| { (*f == b) };
    let mut ret = Vec::<&[u8]>::new();
    for x in src.splitn(max_splits, is_match) {
        ret.push(x);
    }
    return ret;
}


/// Split src on '\r\n' 
pub fn split_bytes_on_crlf(src: &[u8]) -> Vec<&[u8]> {
    let mut start_idx = 0;
    let mut current_idx = 0;
    let mut ret = Vec::<&[u8]>::new();
    for w in src.windows(2) {
        if w == b"\r\n" {
            ret.push(src.slice(start_idx, current_idx));
            start_idx = current_idx + 2;
        }
        current_idx += 1;
    }
    return ret;
}


// ugh, need stdlib lookup table
fn to_hexval(byte: u8) -> Option<u8> {
    match byte {
        b'A'...b'F' => Some(byte - b'A' + 10),
        b'a'...b'f' => Some(byte - b'a' + 10),
        b'0'...b'9' => Some(byte - b'0'),
        _ => None
    }
}


// ugh...slow bounds check
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

pub fn lstrip(input: &[u8]) -> &[u8] {
    let mut pos = 0;
    for c in input.iter() {
        if *c != b' ' {
            break;
        }
        pos += 1;
    }
    return input.slice_from(pos);

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
    let parts = split_bytes_on(a.as_slice(), b' ', 10);
    assert!(parts.len() == 3);
    assert!(parts[0] == b"hello");
    assert!(parts[1] == b"world");
    assert!(parts[2] == b"dude");
}

#[test]
fn test_split_crlf() {
    let a = b"hello world\r\ndude\r\n\r\nlast one\r\n";
    let parts = split_bytes_on_crlf(a.as_slice());
    assert_eq!(parts.len(),  4);
    assert!(parts[0] == b"hello world");
    assert!(parts[1] == b"dude");
    assert!(parts[2] == b"");
    assert!(parts[3] == b"last one");
}

#[test]
fn test_percent_decode() {
    assert!(to_hexval(b'F').unwrap() == 15);
    assert_eq!(percent_decode(b"/hi%20there%ff%00"), b"/hi there\xff\x00");
    assert_eq!(percent_decode(b"/%ff%00%"), b"/\xff\x00%");
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

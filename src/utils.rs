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
    let is_space = |&:f: &u8| { 
        return (*f == b); 
    };
    let mut ret = Vec::<&[u8]>::new();
    for x in src.splitn(max_splits, is_space) {
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

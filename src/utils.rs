
// This really needs to be standard
pub fn memmem(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() == 0 {
        panic!("memmem: empty needle");
    }
    let mut idx = 0us;
    loop {
        let end_idx = idx + needle.len();
        if end_idx > haystack.len() {
            return None;
        }
        if haystack.slice(idx, end_idx) == needle {
            return Some(idx);
        }
        idx += 1;
    }
}

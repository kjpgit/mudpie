/// Escape `s` so it is safe in an HTML element context (NOT attributes)
/// Escapes &, <, and > only.
pub fn html_element_escape(s: &str) -> String {
    let mut ret = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => ret.push_str("&amp;"),
            '<' => ret.push_str("&lt;"),
            '>' => ret.push_str("&gt;"),
            _ => ret.push(c),
        }
    }
    return ret;
}


#[test]
fn test_html_element_escape() {
    assert_eq!(&*html_element_escape("&&<>hi there"),
        "&amp;&amp;&lt;&gt;hi there");
}

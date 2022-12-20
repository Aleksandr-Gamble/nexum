//! The text module contains fairly low-level code for cleaning up and truncating text safely

use unicode_segmentation::UnicodeSegmentation;


/// This function to safely truncate string slices is a modification of this answer:
/// https://stackoverflow.com/questions/38461429/how-can-i-truncate-a-string-to-have-at-most-n-characters
/// ... but with grapheme clusters which is probably what you actually want
pub fn truncate(s: &str, max_chars: usize) -> &str {
    match s.grapheme_indices(true).nth(max_chars) {
        None => s,
        Some((idx, _)) => &s[..idx],
    }
}


/// All Strings in rust are valid UTF-8
/// However, Rust considers a null byte ([0;8]: Vec<u8>) to be valid, whereas Postgres does not!
/// To avoid those errors, you can use this function to remove null utf8 bytes
/// This function consumes the old string and returns it again (unless any changes are made)
/// to avoid having to do more allocation
pub fn remove_null_utf8(s: String) -> String {
    if s.contains(char::from(0)) {
        return s.replace(char::from(0),"")
    }
    s
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        let s1 = "ボルテックス";
        let s1t = truncate(s1, 3);
        assert_eq!(s1t, "ボルテ");
        assert_eq!(s1, truncate(s1, 99));
    }
}
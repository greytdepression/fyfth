pub fn fuzzy_match(haystack: &str, needle: &str) -> bool {
    let haystack = haystack.to_lowercase();
    let needle = needle.to_lowercase();

    let mut needle_chars = needle.chars();
    let mut current_needle_char = needle_chars.next();
    for c_haystack in haystack.chars() {
        let Some(c_needle) = current_needle_char else {
            break;
        };

        if c_needle == c_haystack {
            current_needle_char = needle_chars.next();
        }
    }

    current_needle_char.is_none()
}

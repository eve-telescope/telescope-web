//! `Str::slug`: the title transliterated to ASCII, lower case, anything
//! but letters, digits, spaces and dashes removed, `@` spelled `at`, runs
//! of spaces, dashes and underscores made one dash, no dash at either
//! end. The transliteration is the `deunicode` crate's; where it differs
//! from PHP's (scripts PHP left out), `tests/integration/golden_php.rs`
//! lists the difference.

pub fn slug(title: &str) -> String {
    let ascii = deunicode::deunicode_with_tofu(title, "");
    let title = ascii.replace('_', "-").replace('@', "-at-").to_lowercase();
    let mut slug = String::with_capacity(title.len());
    let mut dash = false;
    for c in title.chars() {
        if c == '-' || c.is_whitespace() {
            dash = true;
        } else if c.is_ascii_alphanumeric() {
            if dash && !slug.is_empty() {
                slug.push('-');
            }
            dash = false;
            slug.push(c);
        }
    }
    slug
}

#[cfg(test)]
mod tests {
    use super::slug;

    #[test]
    fn slugs_like_laravel() {
        assert_eq!(slug("Alpha Intel"), "alpha-intel");
        assert_eq!(slug("  Ørsted & Søn's Ops!  "), "orsted-sons-ops");
        assert_eq!(slug("user@example.com"), "user-at-examplecom");
        assert_eq!(slug("a--b__c"), "a-b-c");
        assert_eq!(slug("!!!"), "");
    }
}

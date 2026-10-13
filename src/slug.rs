//! A network's slug: the title transliterated to ASCII (the `deunicode`
//! crate), lower case, anything but letters, digits, spaces and dashes
//! removed, `@` spelled `at`, runs of spaces, dashes and underscores made
//! one dash, no dash at either end.

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
    fn slugs() {
        for (title, expected) in [
            ("Alpha Intel", "alpha-intel"),
            ("  Ørsted & Søn's Ops!  ", "orsted-sons-ops"),
            ("!!!", ""),
            ("Goonswarm Fédération", "goonswarm-federation"),
            (
                "TEST alliance -- please   ignore",
                "test-alliance-please-ignore",
            ),
            ("user@example.com", "user-at-examplecom"),
            ("情报网络", "qing-bao-wang-luo"),
            ("Ελληνικά", "ellenika"),
            ("Русская разведка", "russkaia-razvedka"),
            ("Ünïcode_under_score", "unicode-under-score"),
            ("a--b__c", "a-b-c"),
            ("100% Pure", "100-pure"),
            ("émigré café", "emigre-cafe"),
            ("ÆØÅ æøå ß", "aeoa-aeoa-ss"),
            ("Đorđe Šćepanović", "dorde-scepanovic"),
            ("日本語 テスト", "ri-ben-yu-tesuto"),
            ("😀 emoji net", "grinning-emoji-net"),
            ("tab\there", "tab-here"),
            ("x/y\\z", "xyz"),
            ("C++ & C#", "c-c"),
            ("O'Brien's", "obriens"),
            ("Zoë-Ägir", "zoe-agir"),
            ("İstanbul", "istanbul"),
            ("ﬁne ligature", "fine-ligature"),
        ] {
            assert_eq!(slug(title), expected, "{title:?}");
        }
    }
}

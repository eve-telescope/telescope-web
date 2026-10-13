//! What Laravel's helpers answered for the inputs a port depends on
//! (tests/fixtures/golden/php-slug.json, scripts/laravel-reference/php-slug.php):
//! `Str::slug`, which names new networks. The slug is only shown, never
//! looked up; the port transliterates with the `deunicode` crate, which
//! also spells out scripts PHP's table left empty (FIXES.md, deliberate
//! changes). Every such difference is listed here.

use crate::common::golden::read_json;

/// Inputs where the crate answers differently, and its answer.
const DIFFERENCES: [(&str, &str); 4] = [
    ("Ελληνικά", "ellenika"),
    ("情报网络", "qing-bao-wang-luo"),
    ("日本語 テスト", "ri-ben-yu-tesuto"),
    ("😀 emoji net", "grinning-emoji-net"),
];

#[test]
fn slugs_match_laravel() {
    let recorded = read_json("golden/php-slug.json");
    let mut mismatches = Vec::new();
    for case in recorded.as_array().expect("cases") {
        let input = case["input"].as_str().expect("an input");
        let expected = DIFFERENCES
            .iter()
            .find(|(different, _)| *different == input)
            .map_or_else(|| case["slug"].as_str().expect("a slug"), |(_, ours)| ours);
        let ours = telescope::slug::slug(input);
        if ours != expected {
            mismatches.push(format!(
                "{input:?}: laravel {:?}, ours {ours:?}",
                case["slug"]
            ));
        }
    }
    assert!(mismatches.is_empty(), "{mismatches:#?}");
}

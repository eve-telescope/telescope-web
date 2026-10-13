// What Str::slug answers for the inputs the slug port is checked with:
//   scripts/laravel-reference/run.sh php "$(cat scripts/laravel-reference/php-slug.php)" > tests/fixtures/golden/php-slug.json
$inputs = ["Alpha Intel", "  Ørsted & Søn's Ops!  ", "!!!", "Goonswarm Fédération", "TEST alliance -- please   ignore", "user@example.com", "情报网络", "Ελληνικά", "Русская разведка", "Ünïcode_under_score", "a--b__c", "100% Pure", "émigré café", "ÆØÅ æøå ß", "Đorđe Šćepanović", "日本語 テスト", "😀 emoji net", "tab\there", "x/y\\z", "C++ & C#", "O'Brien's", "Zoë-Ägir", "İstanbul", "ﬁne ligature"];
$out = [];
foreach ($inputs as $i) { $out[] = ["input" => $i, "slug" => Illuminate\Support\Str::slug($i)]; }
echo json_encode($out, JSON_UNESCAPED_UNICODE | JSON_PRETTY_PRINT);

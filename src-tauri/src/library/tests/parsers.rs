//! The YAML frontmatter and TOML subsets on the shapes library files use.

use crate::library::frontmatter::{parse, Yaml};
use crate::library::tomlish;

#[test]
fn frontmatter_scalars_quotes_blocks_and_lists() {
    let doc = parse(
        "---\r\nname: x # a comment\r\nplain: C# and Rust\r\nsingle: 'it''s'\r\n\
         double: \"a \\\"b\\\" \\u00e9\"\r\nfolded: >-\r\n  one\r\n  two\r\n\r\n  three\r\n\
         literal: |\r\n  line 1\r\n    indented\r\n\
         flow: [a, \"b, c\", 'd']\r\nlist:\r\n  - one\r\n  - \"two\"\r\nbare:\r\n- x\r\n\
         nested:\r\n  author: Ann\r\n  tags: [p, q]\r\nempty:\r\nglobs: **/*.ts\r\n---\r\n\r\nBody\r\n",
    )
    .expect("parse");
    assert!(doc.has_frontmatter);
    assert_eq!(doc.text("name").as_deref(), Some("x"));
    assert_eq!(doc.text("plain").as_deref(), Some("C# and Rust"));
    assert_eq!(doc.text("single").as_deref(), Some("it's"));
    assert_eq!(doc.text("double").as_deref(), Some("a \"b\" é"));
    assert_eq!(doc.text("folded").as_deref(), Some("one two\nthree"));
    assert_eq!(doc.text("literal").as_deref(), Some("line 1\n  indented"));
    assert_eq!(doc.list("flow"), vec!["a", "b, c", "d"]);
    assert_eq!(doc.list("list"), vec!["one", "two"]);
    assert_eq!(doc.list("bare"), vec!["x"]);
    assert_eq!(doc.nested("metadata", "author"), None);
    assert_eq!(doc.nested("nested", "author").as_deref(), Some("Ann"));
    assert_eq!(doc.nested("nested", "tags").as_deref(), Some("p, q"));
    assert_eq!(doc.get("empty"), Some(&Yaml::Str(String::new())));
    assert_eq!(doc.list("globs"), vec!["**/*.ts"]);
    assert_eq!(doc.body, "Body\n");
}

#[test]
fn frontmatter_absent_or_broken() {
    let doc = parse("# Title\n\nText").expect("no frontmatter");
    assert!(!doc.has_frontmatter);
    assert_eq!(doc.body, "# Title\n\nText");
    assert!(parse("---\nname: x\n").is_err(), "not closed");
    assert!(parse("---\nname: \"x\n---\n").is_err(), "unclosed quote");
    assert!(parse("---\n  indented: x\nname: y\n---\n").is_err());
    let plain = parse("---\ndescription: Use when\n  the user asks.\n---\nB").expect("parse");
    assert_eq!(
        plain.text("description").as_deref(),
        Some("Use when the user asks.")
    );
}

#[test]
fn toml_strings_arrays_and_tables() {
    let table = tomlish::parse(
        "# comment\nname = \"a\\tb\" # trailing\nlit = 'C:\\path'\nn = 3\nflag = true\n\
         multi = \"\"\"\nline \"1\"\nline \\\n    2\"\"\"\nraw = '''\nkeep \\n as is\n'''\n\
         list = [\n  \"x\",\n  'y', # c\n]\ninline = { a = 1 }\n\n[tools]\nshell = \"bash\"\n",
    )
    .expect("parse");
    assert_eq!(table.text("name").as_deref(), Some("a\tb"));
    assert_eq!(table.text("lit").as_deref(), Some("C:\\path"));
    assert_eq!(table.text("n").as_deref(), Some("3"));
    assert_eq!(table.text("flag").as_deref(), Some("true"));
    assert_eq!(table.text("multi").as_deref(), Some("line \"1\"\nline 2"));
    assert_eq!(table.text("raw").as_deref(), Some("keep \\n as is"));
    assert_eq!(table.text("list").as_deref(), Some("x, y"));
    assert_eq!(table.text("inline").as_deref(), Some("{ a = 1 }"));
    assert_eq!(table.text("tools.shell").as_deref(), Some("bash"));
    assert!(tomlish::parse("name = \"x").is_err());
    assert!(tomlish::parse("name \"x\"").is_err());
    assert!(tomlish::parse("a = \"\"\"\nnever closed").is_err());
}

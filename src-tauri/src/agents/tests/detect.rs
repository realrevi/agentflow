use crate::agents::detect::detect;
use crate::agents::targets::Target;
use crate::util::test_dir::TestDir;

fn workspace(label: &str, files: &[(&str, &str)]) -> TestDir {
    let dir = TestDir::new(label);
    for (rel, text) in files {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
    dir
}

fn value(dir: &TestDir, key: &str) -> String {
    detect(dir.path())
        .values
        .get(key)
        .cloned()
        .unwrap_or_default()
}

#[test]
fn a_web_project_with_css_variables() {
    let dir = workspace(
        "detect-web",
        &[
            (
                "package.json",
                r#"{ "scripts": { "test": "vitest" },
                     "dependencies": { "react": "19", "react-dom": "19" },
                     "devDependencies": { "vitest": "3", "tailwindcss": "4" } }"#,
            ),
            ("pnpm-lock.yaml", "lockfileVersion: 9"),
            ("src/App.tsx", "export const App = () => null;"),
            ("src/main.tsx", ""),
            (
                "src/styles.css",
                ":root { --color-primary: #1E40AF; --accent: rgb(245, 158, 11); --space-2: 8px; }\n\
                 body { font-family: \"Inter\", system-ui, sans-serif; }\n\
                 @media (min-width: 768px) { body { padding: 0; } }\n\
                 @media (max-width: 40em) { body { margin: 0; } }",
            ),
            (
                "index.html",
                "<link href=\"https://fonts.googleapis.com/css2?family=Poppins:wght@400\">",
            ),
            ("CLAUDE.md", "# rules"),
            (".cursor/rules/style.mdc", "---\n---\n"),
        ],
    );
    assert_eq!(value(&dir, "languages"), "TypeScript, CSS, HTML");
    assert_eq!(value(&dir, "packageManager"), "pnpm");
    assert_eq!(value(&dir, "testFramework"), "Vitest");
    assert_eq!(value(&dir, "testCommand"), "pnpm test");
    assert_eq!(value(&dir, "uiFramework"), "React + Tailwind CSS");
    assert_eq!(
        value(&dir, "colors"),
        "primary #1e40af, accent rgb(245, 158, 11)"
    );
    assert_eq!(value(&dir, "fonts"), "Inter, Poppins");
    assert_eq!(value(&dir, "spacing"), "space-2 8px");
    assert_eq!(value(&dir, "breakpoints"), "640px, 768px");
    let existing: Vec<(Target, String)> = detect(dir.path())
        .existing
        .into_iter()
        .map(|f| (f.target, f.path))
        .collect();
    assert_eq!(
        existing,
        [
            (Target::ClaudeCode, "CLAUDE.md".to_owned()),
            (Target::Cursor, ".cursor/rules/style.mdc".to_owned())
        ]
    );
}

#[test]
fn rust_python_and_dotnet_projects() {
    let rust = workspace(
        "detect-rust",
        &[
            ("Cargo.toml", "[package]\nname = \"x\""),
            ("src/main.rs", "fn main() {}"),
        ],
    );
    assert_eq!(value(&rust, "languages"), "Rust");
    assert_eq!(value(&rust, "packageManager"), "cargo");
    assert_eq!(value(&rust, "testCommand"), "cargo test");

    let python = workspace(
        "detect-python",
        &[
            (
                "pyproject.toml",
                "[project]\ndependencies = []\n[tool.pytest.ini_options]\n",
            ),
            ("app/main.py", "print(1)"),
        ],
    );
    assert_eq!(value(&python, "packageManager"), "pip");
    assert_eq!(value(&python, "testFramework"), "pytest");

    let dotnet = workspace(
        "detect-dotnet",
        &[
            (
                "App.Tests/App.Tests.csproj",
                "<PackageReference Include=\"xunit\" />",
            ),
            ("App.Tests/UnitTest1.cs", "class T {}"),
        ],
    );
    assert_eq!(value(&dotnet, "packageManager"), "dotnet");
    assert_eq!(value(&dotnet, "testFramework"), "xUnit");
    assert_eq!(value(&dotnet, "testCommand"), "dotnet test");
}

#[test]
fn a_tailwind_config_and_an_empty_folder() {
    let tailwind = workspace(
        "detect-tailwind",
        &[(
            "tailwind.config.js",
            "module.exports = { theme: { extend: {\n  colors: { brand: '#FF0000', 'brand-dark': '#990000' },\n  fontFamily: { sans: ['Satoshi', 'sans-serif'] } } } }",
        )],
    );
    assert_eq!(
        value(&tailwind, "colors"),
        "brand #ff0000, brand-dark #990000"
    );
    assert_eq!(value(&tailwind, "fonts"), "Satoshi");
    assert_eq!(
        value(&tailwind, "breakpoints"),
        "sm 640px, md 768px, lg 1024px, xl 1280px, 2xl 1536px"
    );

    let empty = TestDir::new("detect-empty");
    let found = detect(empty.path());
    assert!(found.values.is_empty() && found.existing.is_empty());
}

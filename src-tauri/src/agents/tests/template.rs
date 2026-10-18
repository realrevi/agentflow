use serde_json::{json, Value};

use super::sample;
use crate::agent::tools::TOOL_NAMES;
use crate::agents::detect::DETECTORS;
use crate::agents::placeholders::{placeholders, render, values};
use crate::agents::starters::starters;
use crate::agents::template::{Answers, Template};
use crate::agents::validate::{validate, ID_PATTERN, QUESTION_ID_PATTERN, VERSION_PATTERN};

type Change = Box<dyn FnOnce(&mut Value)>;

fn with(change: impl FnOnce(&mut Value)) -> Vec<String> {
    let mut value = serde_json::to_value(sample()).expect("json");
    change(&mut value);
    match serde_json::from_value::<Template>(value) {
        Ok(template) => validate(&template),
        Err(err) => vec![format!("parse: {err}")],
    }
}

#[test]
fn the_sample_and_every_starter_are_valid() {
    assert_eq!(validate(&sample()), Vec::<String>::new());
    let all = starters();
    assert_eq!(all.len(), 9);
    for template in all {
        assert_eq!(validate(&template), Vec::<String>::new(), "{}", template.id);
    }
}

#[test]
fn invalid_templates_say_what_is_wrong() {
    let cases: Vec<(Change, &str)> = vec![
        (Box::new(|v| v["schema"] = json!(2)), "schema"),
        (Box::new(|v| v["id"] = json!("../evil")), "id:"),
        (Box::new(|v| v["id"] = json!("Upper")), "id:"),
        (Box::new(|v| v["version"] = json!("1.0")), "version"),
        (
            Box::new(|v| v["description"] = json!("two\nlines")),
            "description",
        ),
        (
            Box::new(|v| v["instructions"] = json!("Hi {{nope}}")),
            "unknown placeholder {{nope}}",
        ),
        (
            Box::new(|v| v["instructions"] = json!("<!-- agentflow:end x -->")),
            "must not contain",
        ),
        (
            Box::new(|v| v["questions"][0]["detect"] = json!("weather")),
            "unknown detector",
        ),
        (
            Box::new(|v| v["questions"][1]["id"] = json!("colors")),
            "used twice",
        ),
        (
            Box::new(|v| v["questions"][2]["default"] = json!(["c"])),
            "does not fit",
        ),
        (
            Box::new(|v| v["questions"][2]["options"] = json!([])),
            "options",
        ),
        (
            Box::new(|v| v["questions"][0]["id"] = json!("name")),
            "built-in",
        ),
        (
            Box::new(|v| v["targets"] = json!({ "agentflow": { "tools": ["rm_rf"] } })),
            "unknown tool",
        ),
        (Box::new(|v| v["surprise"] = json!(1)), "parse"),
    ];
    for (change, expected) in cases {
        let errors = with(change);
        assert!(
            errors.iter().any(|e| e.contains(expected)),
            "expected {expected:?} in {errors:?}"
        );
    }
}

#[test]
fn placeholders_render_answers_defaults_and_drop_empty_lines() {
    let template = sample();
    let mut answers = Answers::new();
    answers.insert("colors".into(), json!("primary #112233"));
    let values = values(&template, &answers, "shop").expect("values");
    let text = render(&template.instructions, &values).expect("render");
    assert_eq!(
        text,
        "You help shop.\nColors: primary #112233\nStrict: yes\nFocus: a"
    );
    // No answer and no default: the line goes away.
    let values = values_for(&template, Answers::new());
    let text = render(&template.instructions, &values).expect("render");
    assert!(!text.contains("Colors:"), "{text}");
    assert_eq!(placeholders("{{ a }} {{b_1}} {{1x}} {{}}"), ["a", "b_1"]);
}

fn values_for(template: &Template, answers: Answers) -> std::collections::BTreeMap<String, String> {
    values(template, &answers, "w").expect("values")
}

#[test]
fn placeholder_errors_unknown_missing_and_wrong_shape() {
    let mut template = sample();
    let filled = values_for(&template, Answers::new());
    assert!(render("{{unknown}}", &filled)
        .unwrap_err()
        .to_string()
        .contains("unknown placeholder {{unknown}}"));
    template.questions[0].required = true;
    let err = values(&template, &Answers::new(), "w")
        .unwrap_err()
        .to_string();
    assert!(err.contains("an answer is needed for \"Colors\""), "{err}");
    let mut answers = Answers::new();
    answers.insert("focus".into(), json!(["z"]));
    assert!(values(&template, &answers, "w").is_err(), "not an option");
    answers.insert("focus".into(), json!("a"));
    assert!(
        values(&template, &answers, "w").is_err(),
        "a string for multiChoice"
    );
    let mut marker = Answers::new();
    marker.insert("colors".into(), json!("<!-- agentflow:end demo-agent -->"));
    assert!(
        values(&sample(), &marker, "w").is_err(),
        "answers cannot fake a marker"
    );
}

/// schemas/agent-template.schema.json documents the same rules as validate.rs.
#[test]
fn the_json_schema_matches_the_rust_rules() {
    let schema: Value = serde_json::from_str(include_str!(
        "../../../../schemas/agent-template.schema.json"
    ))
    .expect("schema json");
    let props = &schema["properties"];
    assert_eq!(props["id"]["pattern"], ID_PATTERN);
    assert_eq!(props["version"]["pattern"], VERSION_PATTERN);
    let question = &schema["$defs"]["question"]["properties"];
    assert_eq!(question["id"]["pattern"], QUESTION_ID_PATTERN);
    let detectors: Vec<&str> = question["detect"]["enum"]
        .as_array()
        .expect("detect enum")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(detectors, DETECTORS);
    let tools: Vec<&str> = props["targets"]["properties"]["agentflow"]["properties"]["tools"]
        ["items"]["enum"]
        .as_array()
        .expect("tools enum")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    assert_eq!(tools, TOOL_NAMES);
    let kinds: Vec<&str> = question["type"]["enum"]
        .as_array()
        .expect("types")
        .iter()
        .filter_map(Value::as_str)
        .collect();
    for kind in &kinds {
        let parsed: Result<crate::agents::template::QuestionKind, _> =
            serde_json::from_value(json!(kind));
        assert!(parsed.is_ok(), "{kind}");
    }
    assert_eq!(kinds.len(), 7);
    assert_eq!(
        props["instructions"]["maxLength"],
        crate::agents::validate::MAX_INSTRUCTIONS
    );
    for kind in props["kind"]["enum"].as_array().expect("kinds") {
        let parsed: Result<crate::agents::template::TemplateKind, _> =
            serde_json::from_value(kind.clone());
        assert!(parsed.is_ok(), "{kind}");
    }
}

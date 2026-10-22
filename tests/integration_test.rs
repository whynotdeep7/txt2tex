//! End-to-end checks over the parse → render pipeline.
//!
//! Unit tests live beside the code they cover; these are here to pin the
//! behaviour a user would notice, using input in the shape people actually
//! write it.

use txt2tex::model::Resume;
use txt2tex::parser::parse_content;
use txt2tex::template::render;
use txt2tex::TEMPLATE;

fn tex_for(input: &str) -> String {
    render(&parse_content(input))
}

// ── parsing ───────────────────────────────────────────────────

#[test]
fn parses_personal_information() {
    let resume = parse_content("# PERSONAL INFORMATION\nname: Alex\nemail: alex@test.com\n");
    assert_eq!(resume.name, "Alex");
    assert_eq!(resume.email, "alex@test.com");
}

#[test]
fn parses_skills_into_categories() {
    let resume =
        parse_content("# SKILLS\nLanguages: Rust, C++, Python\nFrameworks: React, Node.js\n");
    assert_eq!(resume.skills.len(), 2);
    assert_eq!(resume.skills[0].category, "Languages");
    assert_eq!(resume.skills[0].items, ["Rust", "C++", "Python"]);
    assert_eq!(resume.skills[1].items, ["React", "Node.js"]);
}

#[test]
fn parses_a_project() {
    let resume = parse_content(
        "\
# PROJECTS
project:
title: MyApp
description: A cool app I built
tech: Rust, Node.js
",
    );
    assert_eq!(resume.projects.len(), 1);
    assert_eq!(resume.projects[0].title, "MyApp");
    assert_eq!(resume.projects[0].tech, ["Rust", "Node.js"]);
}

#[test]
fn parses_multiple_projects() {
    let resume = parse_content(
        "\
# PROJECTS
project:
title: Project A
tech: Rust

project:
title: Project B
tech: Python
",
    );
    assert_eq!(resume.projects.len(), 2);
    assert_eq!(resume.projects[0].title, "Project A");
    assert_eq!(resume.projects[1].title, "Project B");
}

#[test]
fn parses_experience_bullets() {
    let resume = parse_content(
        "\
# EXPERIENCE
experience:
company: TechCorp
role: Intern
duration: Summer 2024
- Built REST APIs handling 10k requests per day
- Reduced latency by 30%
",
    );
    assert_eq!(resume.experience[0].company, "TechCorp");
    assert_eq!(resume.experience[0].details.len(), 2);
    assert_eq!(
        resume.experience[0].details[0],
        "Built REST APIs handling 10k requests per day"
    );
}

#[test]
fn a_resume_needs_a_name_and_an_email() {
    assert!(parse_content("# PERSONAL INFORMATION\nname: Alex\nemail: a@b.com\n").is_valid());
    assert!(!parse_content("# PERSONAL INFORMATION\nname: Alex\n").is_valid());
    assert!(!parse_content("# PERSONAL INFORMATION\nemail: a@b.com\n").is_valid());
}

// ── rendering ─────────────────────────────────────────────────

#[test]
fn renders_a_complete_document() {
    let tex = tex_for("# PERSONAL INFORMATION\nname: Alex Johnson\nemail: a@b.com\n");
    assert!(tex.contains(r"\documentclass"));
    assert!(tex.contains(r"\begin{document}"));
    assert!(tex.contains(r"\end{document}"));
    assert!(tex.contains("Alex Johnson"));
}

#[test]
fn renders_the_skills_section() {
    let tex = tex_for(
        "# PERSONAL INFORMATION\nname: Alex\nemail: a@b.com\n# SKILLS\nLanguages: Rust, Python\n",
    );
    assert!(tex.contains("Technical Skills"));
    assert!(tex.contains("Languages"));
    assert!(tex.contains("Rust, Python"));
}

// ── LaTeX escaping ────────────────────────────────────────────
//
// Each of these is a character that silently corrupts the PDF, or fails the
// build outright, if it reaches the .tex file as itself.

#[test]
fn escapes_special_characters_in_user_text() {
    assert!(tex_for("# PERSONAL INFORMATION\nname: John & Jane\n").contains(r"John \& Jane"));
    assert!(tex_for("# PERSONAL INFORMATION\nname: John_Doe\n").contains(r"John\_Doe"));
    assert!(tex_for("# PERSONAL INFORMATION\nname: Alex {Smith}\n").contains(r"Alex \{Smith\}"));
    assert!(
        tex_for("# PERSONAL INFORMATION\nname: Test~User\n").contains(r"Test\textasciitilde{}User")
    );
    assert!(
        tex_for("# PERSONAL INFORMATION\nname: Alex^123\n").contains(r"Alex\textasciicircum{}123")
    );
}

/// Regression: the old escaper chained `str::replace`, so the `{}` it inserted
/// for a backslash got escaped again into `\textbackslash\{\}`.
#[test]
fn escaping_a_backslash_does_not_escape_its_own_braces() {
    let tex = tex_for(r"# PERSONAL INFORMATION\nname: A\B");
    assert!(!tex.contains(r"\textbackslash\{\}"));
}

/// A GitHub handle with an underscore has to be escaped in the visible label
/// but left alone inside the href target, or the link points nowhere.
#[test]
fn github_links_escape_the_label_but_not_the_url() {
    let tex = tex_for(
        "# PERSONAL INFORMATION\nname: Alex\nemail: a@b.com\ngithub: github.com/user_name\n",
    );
    assert!(
        tex.contains(r"\href{https://github.com/user_name}"),
        "url must stay literal:\n{tex}"
    );
    assert!(
        tex.contains(r"{github.com/user\_name}"),
        "label must be escaped:\n{tex}"
    );
}

/// Regression: `split('.')` shattered every dotted word, so the shipped
/// template rendered "Built with Node" / "js and PostgreSQL" as two bullets.
#[test]
fn dotted_words_survive_the_sentence_split() {
    let tex = tex_for(
        "\
# PROJECTS
project:
title: T
description: Built a backend with Node.js and PostgreSQL. Cut latency to 1.5s
tech: Rust
",
    );
    assert!(
        tex.contains(r"\resumeItem{Built a backend with Node.js and PostgreSQL}"),
        "{tex}"
    );
    assert!(tex.contains(r"\resumeItem{Cut latency to 1.5s}"), "{tex}");
    assert!(!tex.contains(r"\resumeItem{js and PostgreSQL}"));
}

// ── the shipped template ──────────────────────────────────────

/// Whatever else breaks, `txt2tex new` must produce something that builds.
#[test]
fn the_shipped_template_parses_into_a_valid_resume() {
    let resume: Resume = parse_content(TEMPLATE);
    assert!(resume.is_valid());
    assert_eq!(resume.experience.len(), 3);
    assert_eq!(resume.projects.len(), 3);
    assert_eq!(resume.skills.len(), 4);
}

/// The template is the demo. It should not trip the tool's own advice.
#[test]
fn the_shipped_template_raises_no_issues() {
    let findings = txt2tex::analyzer::analyze(&parse_content(TEMPLATE));
    let issues: Vec<_> = findings
        .iter()
        .filter(|f| f.severity == txt2tex::analyzer::Severity::Issue)
        .map(|f| f.message.as_str())
        .collect();
    assert!(issues.is_empty(), "template has issues: {issues:#?}");
}

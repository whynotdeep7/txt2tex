//! Renders a [`Resume`] as a LaTeX document.
//!
//! The preamble is the widely-used "Jake's Resume" layout; everything after it
//! is generated from the model. Every piece of user text passes through
//! [`crate::latex`] on the way in — nothing reaches the `.tex` file unescaped.

use crate::latex::{escape, escape_url};
use crate::model::Resume;

const PREAMBLE: &str = r#"\documentclass[letterpaper,11pt]{article}

\usepackage{latexsym}
\usepackage[empty]{fullpage}
\usepackage{titlesec}
\usepackage{marvosym}
\usepackage[usenames,dvipsnames]{color}
\usepackage{verbatim}
\usepackage{enumitem}
\usepackage[hidelinks]{hyperref}
\usepackage{fancyhdr}
\usepackage[english]{babel}
\usepackage{tabularx}

\pagestyle{fancy}
\fancyhf{}
\fancyfoot{}
\renewcommand{\headrulewidth}{0pt}
\renewcommand{\footrulewidth}{0pt}

\addtolength{\oddsidemargin}{-0.5in}
\addtolength{\evensidemargin}{-0.5in}
\addtolength{\textwidth}{1in}
\addtolength{\topmargin}{-.5in}
\addtolength{\textheight}{1.0in}

\urlstyle{same}
\raggedbottom
\raggedright
\setlength{\tabcolsep}{0in}

\titleformat{\section}{
  \vspace{-4pt}\scshape\raggedright\large
}{}{0em}{}[\color{black}\titlerule \vspace{-5pt}]

\newcommand{\resumeItem}[1]{
  \item\small{{#1 \vspace{-2pt}}}
}

\newcommand{\resumeSubheading}[4]{
  \vspace{-2pt}\item
    \begin{tabular*}{0.97\textwidth}[t]{l@{\extracolsep{\fill}}r}
      \textbf{#1} & #2 \\
      \textit{\small#3} & \textit{\small #4} \\
    \end{tabular*}\vspace{-7pt}
}

\newcommand{\resumeProjectHeading}[2]{
    \item
    \begin{tabular*}{0.97\textwidth}{l@{\extracolsep{\fill}}r}
      \small#1 & #2 \\
    \end{tabular*}\vspace{-7pt}
}

\renewcommand\labelitemii{$\vcenter{\hbox{\tiny$\bullet$}}$}

\newcommand{\resumeSubHeadingListStart}{\begin{itemize}[leftmargin=0.15in, label={}]}
\newcommand{\resumeSubHeadingListEnd}{\end{itemize}}
\newcommand{\resumeItemListStart}{\begin{itemize}}
\newcommand{\resumeItemListEnd}{\end{itemize}\vspace{-5pt}}

\begin{document}
"#;

/// Infallible: an empty resume renders an empty (but valid) document.
pub fn render(resume: &Resume) -> String {
    let mut out = String::from(PREAMBLE);
    heading(&mut out, resume);
    education(&mut out, resume);
    experience(&mut out, resume);
    projects(&mut out, resume);
    skills(&mut out, resume);
    out.push_str("\\end{document}\n");
    out
}

fn heading(out: &mut String, resume: &Resume) {
    out.push_str("\n\\begin{center}\n");
    out.push_str(&format!(
        "    \\textbf{{\\Huge \\scshape {}}} \\\\ \\vspace{{1pt}}\n",
        escape(&resume.name)
    ));

    // Only the contact details that exist, joined by a bar. Building the list
    // first keeps the separators right however many are present.
    let mut contact = Vec::new();
    if !resume.phone.is_empty() {
        contact.push(escape(&resume.phone));
    }
    if !resume.email.is_empty() {
        contact.push(format!(
            "\\href{{mailto:{}}}{{{}}}",
            escape_url(&resume.email),
            escape(&resume.email)
        ));
    }
    if !resume.github.is_empty() {
        let (target, label) = link(&resume.github);
        contact.push(format!(
            "\\href{{{}}}{{{}}}",
            escape_url(&target),
            escape(&label)
        ));
    }

    if !contact.is_empty() {
        out.push_str("    \\small ");
        out.push_str(&contact.join(" $|$ "));
        out.push('\n');
    }
    out.push_str("\\end{center}\n\n");
}

/// Splits a hand-written link into its href target and its visible label.
///
/// People write `github.com/alex` about as often as `https://github.com/alex`.
/// Both have to produce a working link, and neither should show the scheme in
/// the PDF.
fn link(raw: &str) -> (String, String) {
    let label = raw
        .trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    (format!("https://{label}"), label.to_string())
}

fn education(out: &mut String, resume: &Resume) {
    if resume.degree.is_empty() && resume.university.is_empty() {
        return;
    }
    out.push_str("\\section{Education}\n  \\resumeSubHeadingListStart\n");
    out.push_str(&subheading(
        &resume.university,
        "",
        &resume.degree,
        &resume.year,
    ));
    out.push_str("  \\resumeSubHeadingListEnd\n\n");
}

fn experience(out: &mut String, resume: &Resume) {
    if resume.experience.is_empty() {
        return;
    }
    out.push_str("\\section{Experience}\n  \\resumeSubHeadingListStart\n");
    for job in &resume.experience {
        out.push_str(&subheading(&job.company, "", &job.role, &job.duration));
        bullets(
            out,
            &job.details.iter().map(String::as_str).collect::<Vec<_>>(),
            "      ",
        );
        out.push('\n');
    }
    out.push_str("  \\resumeSubHeadingListEnd\n\n");
}

fn projects(out: &mut String, resume: &Resume) {
    if resume.projects.is_empty() {
        return;
    }
    out.push_str("\\section{Projects}\n    \\resumeSubHeadingListStart\n");
    for project in &resume.projects {
        // The bar separates title from tech, so with no tech it would dangle.
        let tech = if project.tech.is_empty() {
            String::new()
        } else {
            format!(" $|$ \\emph{{{}}}", escape(&project.tech.join(", ")))
        };
        out.push_str(&format!(
            "      \\resumeProjectHeading\n          {{\\textbf{{{}}}{}}}{{}}\n",
            escape(&project.title),
            tech
        ));
        bullets(out, &project.bullets(), "          ");
        out.push('\n');
    }
    out.push_str("    \\resumeSubHeadingListEnd\n\n");
}

fn skills(out: &mut String, resume: &Resume) {
    if resume.skills.is_empty() {
        return;
    }
    out.push_str("\\section{Technical Skills}\n");
    out.push_str(" \\begin{itemize}[leftmargin=0.15in, label={}]\n    \\small{\\item{\n");
    for group in &resume.skills {
        out.push_str(&format!(
            "     \\textbf{{{}}}{{: {}}} \\\\\n",
            escape(&group.category),
            escape(&group.items.join(", "))
        ));
    }
    out.push_str("    }}\n \\end{itemize}\n\n");
}

/// The preamble's `\resumeSubheading{title}{right}{subtitle}{subright}`.
fn subheading(title: &str, right: &str, subtitle: &str, subright: &str) -> String {
    format!(
        "    \\resumeSubheading\n      {{{}}}{{{}}}\n      {{{}}}{{{}}}\n",
        escape(title),
        escape(right),
        escape(subtitle),
        escape(subright)
    )
}

/// An `itemize` block, or nothing at all — `\begin{itemize}` with no `\item`
/// is a LaTeX error, so an entry with no bullets must emit no list.
fn bullets(out: &mut String, items: &[&str], indent: &str) {
    let items: Vec<&&str> = items
        .iter()
        .filter(|item| !item.trim().is_empty())
        .collect();
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("{indent}\\resumeItemListStart\n"));
    for item in items {
        out.push_str(&format!("{indent}  \\resumeItem{{{}}}\n", escape(item)));
    }
    out.push_str(&format!("{indent}\\resumeItemListEnd\n"));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Experience, Project};

    #[test]
    fn link_accepts_bare_and_full_urls() {
        assert_eq!(
            link("github.com/alex"),
            ("https://github.com/alex".into(), "github.com/alex".into())
        );
        assert_eq!(
            link("https://github.com/alex"),
            ("https://github.com/alex".into(), "github.com/alex".into())
        );
    }

    /// Everything after `\begin{document}`. The preamble defines the same
    /// macro names it uses, so assertions about output have to skip it.
    fn body(tex: &str) -> &str {
        tex.split_once(r"\begin{document}")
            .expect("rendered document")
            .1
    }

    /// `\begin{itemize}` with no `\item` inside is a hard LaTeX error, so an
    /// entry with no bullets must not open a list at all.
    #[test]
    fn entry_without_bullets_emits_no_itemize() {
        let resume = Resume {
            experience: vec![Experience {
                company: "Acme".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let tex = render(&resume);
        assert!(tex.contains("Acme"));
        assert!(!body(&tex).contains(r"\resumeItemListStart"));
    }

    #[test]
    fn empty_resume_still_renders_a_valid_document() {
        let tex = render(&Resume::default());
        assert!(tex.contains(r"\begin{document}"));
        assert!(tex.contains(r"\end{document}"));
    }

    #[test]
    fn project_without_tech_has_no_dangling_bar() {
        let resume = Resume {
            projects: vec![Project {
                title: "T".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(body(&render(&resume)).contains(r"{\textbf{T}}{}"));
    }

    #[test]
    fn project_description_becomes_one_bullet_per_sentence() {
        let resume = Resume {
            projects: vec![Project {
                title: "T".into(),
                description: "Built with Node.js. Served 10k users.".into(),
                ..Default::default()
            }],
            ..Default::default()
        };
        let tex = render(&resume);
        assert!(tex.contains(r"\resumeItem{Built with Node.js}"));
        assert!(tex.contains(r"\resumeItem{Served 10k users}"));
    }
}

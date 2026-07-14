use crate::models::{ExportResult, SESSION_SCHEMA_VERSION, SessionState};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use std::{fs, io::Write, path::Path};
use typst_as_lib::{TypstEngine, typst_kit_options::TypstKitFontOptions};

const STATE_START: &str = "<!-- AGENTIC_COUNCIL_STATE_BASE64:";
const STATE_END: &str = ":END_AGENTIC_COUNCIL_STATE -->";

pub fn export_markdown(session: &SessionState, path: &Path) -> Result<ExportResult, String> {
    require_extension(path, "md")?;
    let markdown = render_markdown(session)?;
    atomic_write(path, markdown.as_bytes())?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        bytes: markdown.len() as u64,
    })
}

pub fn render_markdown(session: &SessionState) -> Result<String, String> {
    let state = serde_json::to_vec(session).map_err(|error| error.to_string())?;
    let encoded = BASE64.encode(state);
    let mut out = String::new();
    out.push_str("---\n");
    out.push_str(&format!(
        "agentic_council_schema: {}\n",
        session.schema_version
    ));
    out.push_str(&format!("session_id: {}\n", session.id));
    out.push_str(&format!("phase: {:?}\n", session.phase).to_ascii_lowercase());
    out.push_str(&format!(
        "created_at: {}\n",
        session.created_at.to_rfc3339()
    ));
    out.push_str(&format!(
        "updated_at: {}\n",
        session.updated_at.to_rfc3339()
    ));
    out.push_str("agents:\n");
    for agent in &session.agents {
        out.push_str(&format!("  - id: {:?}\n    role: {:?}\n    provider: {:?}\n    model: {:?}\n    persona: {:?}\n", agent.id, agent.role, agent.provider_id, agent.model_id, agent.persona_id));
    }
    out.push_str("---\n\n");
    out.push_str(&format!("{STATE_START}{encoded}{STATE_END}\n\n"));
    out.push_str("# Agentic Council Report\n\n");
    out.push_str("## Objective\n\n");
    out.push_str(&session.objective);
    out.push_str("\n\n## Approved aspects\n\n");
    for aspect in &session.aspects {
        out.push_str(&format!(
            "- **{}** (weight {:.2}): {}\n",
            aspect.name, aspect.weight, aspect.description
        ));
    }
    for round in &session.rounds {
        out.push_str(&format!("\n## Round {}\n\n", round.index));
        for response in &round.responses {
            let agent = session
                .agents
                .iter()
                .find(|agent| agent.id == response.agent_id);
            out.push_str(&format!(
                "### {}\n\n",
                agent
                    .map(|value| value.display_name.as_str())
                    .unwrap_or("Unknown agent")
            ));
            out.push_str(&format!("_Provider/model: {}/{} · Status: {:?} · Input tokens: {} · Output tokens: {} · Latency: {}ms_\n\n", agent.map(|value| value.provider_id.as_str()).unwrap_or("unknown"), agent.map(|value| value.model_id.as_str()).unwrap_or("unknown"), response.status, option_number(response.input_tokens), option_number(response.output_tokens), response.latency_ms));
            out.push_str(&response.content);
            out.push_str("\n\n");
        }
        out.push_str("### Moderator friction\n\n");
        for friction in &round.friction {
            out.push_str(&format!(
                "- **{:?}** — {} _Challenge:_ {}\n",
                friction.kind, friction.explanation, friction.challenge
            ));
        }
        out.push_str("\n### Aggregated peer scores\n\n| Agent | Aspect | Median / 10 | Votes / 10 |\n|---|---|---:|---|\n");
        for score in &round.scores {
            let agent = session
                .agents
                .iter()
                .find(|agent| agent.id == score.agent_id)
                .map(|agent| agent.display_name.as_str())
                .unwrap_or("Unknown");
            let aspect = session
                .aspects
                .iter()
                .find(|aspect| aspect.id == score.aspect_id)
                .map(|aspect| aspect.name.as_str())
                .unwrap_or("Unknown");
            let votes = score
                .votes
                .iter()
                .map(|vote| {
                    format!(
                        "{}: {:.1}{}",
                        vote.voter_alias,
                        vote.score,
                        if vote.outlier { " (outlier)" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            out.push_str(&format!(
                "| {} | {} | {:.1} | {} |\n",
                escape_markdown_cell(agent),
                escape_markdown_cell(aspect),
                score.median,
                escape_markdown_cell(&votes)
            ));
        }
    }
    out.push_str(
        "\n---\n_Generated locally by Agentic Council. This report contains no API credentials._\n",
    );
    Ok(out)
}

pub fn import_markdown(path: &Path) -> Result<SessionState, String> {
    require_extension(path, "md")?;
    let content = fs::read_to_string(path)
        .map_err(|error| format!("Session file could not be read: {error}"))?;
    if content.len() > 100 * 1024 * 1024 {
        return Err("Session import exceeds the 100 MiB safety limit.".into());
    }
    let start = content.find(STATE_START).ok_or_else(|| {
        "This Markdown file does not contain restorable Agentic Council state.".to_string()
    })? + STATE_START.len();
    let relative_end = content[start..]
        .find(STATE_END)
        .ok_or_else(|| "The embedded session state marker is incomplete.".to_string())?;
    let encoded = &content[start..start + relative_end];
    let bytes = BASE64
        .decode(encoded.trim())
        .map_err(|_| "The embedded session state is not valid base64.".to_string())?;
    let session: SessionState = serde_json::from_slice(&bytes)
        .map_err(|error| format!("The embedded session state is malformed: {error}"))?;
    if session.schema_version != SESSION_SCHEMA_VERSION {
        return Err(format!(
            "Session schema {} is not supported by this version ({}).",
            session.schema_version, SESSION_SCHEMA_VERSION
        ));
    }
    Ok(session)
}

pub fn export_pdf(session: &SessionState, path: &Path) -> Result<ExportResult, String> {
    require_extension(path, "pdf")?;
    let source = render_typst(session);
    let engine = TypstEngine::builder()
        .main_file(source)
        .search_fonts_with(
            TypstKitFontOptions::default()
                // Keep the embedded serif as the stable primary face, while
                // allowing platform emoji/CJK fonts to fill glyphs it lacks.
                .include_system_fonts(true)
                .include_embedded_fonts(true),
        )
        .build();
    let document = engine
        .compile()
        .output
        .map_err(|error| format!("Typst compilation failed: {error}"))?;
    let pdf = typst_pdf::pdf(&document, &Default::default())
        .map_err(|error| format!("PDF rendering failed: {error:?}"))?;
    atomic_write(path, &pdf)?;
    Ok(ExportResult {
        path: path.to_string_lossy().into_owned(),
        bytes: pdf.len() as u64,
    })
}

fn render_typst(session: &SessionState) -> String {
    let mut body = String::new();
    body.push_str("#set page(paper: \"a4\", margin: (x: 22mm, y: 20mm), numbering: \"1 / 1\")\n#set text(font: \"Libertinus Serif\", size: 10pt, fill: rgb(\"24304a\"))\n#set heading(numbering: \"1.\")\n#set par(justify: true, leading: 0.72em)\n#set page(fill: rgb(\"fbfcff\"))\n#show heading: set text(fill: rgb(\"5848b8\"))\n");
    body.push_str("#align(center)[#text(size: 24pt, weight: \"bold\", fill: rgb(\"332879\"))[Agentic Council] #linebreak() #text(size: 9pt, fill: rgb(\"71809b\"))[Private multi-agent deliberation report]]\n#v(10mm)\n");
    body.push_str(&format!(
        "= Objective\n#text({})\n\n",
        typst_string(&session.objective)
    ));
    body.push_str("= Approved aspects\n");
    for aspect in &session.aspects {
        body.push_str(&format!(
            "- #text(weight: \"bold\")[#text({})] (weight {:.2}): #text({})\n",
            typst_string(&aspect.name),
            aspect.weight,
            typst_string(&aspect.description)
        ));
    }
    for round in &session.rounds {
        body.push_str(&format!("\n= Round {}\n", round.index));
        for response in &round.responses {
            let agent = session
                .agents
                .iter()
                .find(|agent| agent.id == response.agent_id);
            let name = agent
                .map(|agent| agent.display_name.as_str())
                .unwrap_or("Unknown agent");
            body.push_str(&format!(
                "== #text({})\n#text(size: 8pt, fill: rgb(\"71809b\"))[#text({})]\n\n#text({})\n",
                typst_string(name),
                typst_string(&format!(
                    "{} / {} • {:?} • {}ms",
                    agent
                        .map(|value| value.provider_id.as_str())
                        .unwrap_or("unknown"),
                    agent
                        .map(|value| value.model_id.as_str())
                        .unwrap_or("unknown"),
                    response.status,
                    response.latency_ms
                )),
                typst_string(&response.content)
            ));
        }
        body.push_str("\n== Moderator friction\n");
        for item in &round.friction {
            body.push_str(&format!("#block(inset: 8pt, radius: 4pt, fill: rgb(\"f2efff\"))[#text(weight: \"bold\")[#text({})] #text(fill: rgb(\"48536b\"))[#text({})] #linebreak() #emph[#text({})]]\n#v(3pt)\n", typst_string(&format!("{:?}", item.kind)), typst_string(&item.explanation), typst_string(&format!("Challenge: {}", item.challenge))));
        }
        body.push_str("\n== Peer score matrix\n#text(size: 8pt, fill: rgb(\"71809b\"))[Each answer is scored from 0 to 10. The displayed result is the median of all eligible peer votes; models never score themselves.]\n#v(3pt)\n#table(columns: (1.1fr, 1.1fr, .55fr, 1.6fr), inset: 5pt, stroke: rgb(\"dde2ef\"), [*Agent*], [*Aspect*], [*Median / 10*], [*Peer votes / 10*],\n");
        for score in &round.scores {
            let agent = session
                .agents
                .iter()
                .find(|agent| agent.id == score.agent_id)
                .map(|agent| agent.display_name.as_str())
                .unwrap_or("Unknown");
            let aspect = session
                .aspects
                .iter()
                .find(|aspect| aspect.id == score.aspect_id)
                .map(|aspect| aspect.name.as_str())
                .unwrap_or("Unknown");
            let votes = score
                .votes
                .iter()
                .map(|vote| {
                    format!(
                        "{}: {:.1}{}",
                        vote.voter_alias,
                        vote.score,
                        if vote.outlier { " (outlier)" } else { "" }
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            body.push_str(&format!(
                "[#text({})], [#text({})], [{:.1}], [#text({})],\n",
                typst_string(agent),
                typst_string(aspect),
                score.median,
                typst_string(&votes)
            ));
        }
        body.push_str(")\n");
    }
    if let Some(synthesis) = session
        .final_synthesis
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        body.push_str(&format!(
            "\n= Final synthesis\n#text({})\n",
            typst_string(synthesis)
        ));
    }
    body.push_str("\n#v(8mm)\n#line(length: 100%, stroke: rgb(\"dde2ef\"))\n#align(center)[#text(size: 7.5pt, fill: rgb(\"71809b\"))[Generated locally by Agentic Council. No API credentials are included. #linebreak() Developed by #text(weight: \"bold\")[Parsa Farshadfar].]]\n");
    body
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Export path has no parent directory.".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    atomicwrites::AtomicFile::new(path, atomicwrites::AllowOverwrite)
        .write(|file| {
            file.write_all(bytes)?;
            file.sync_all()
        })
        .map_err(|error| error.to_string())
}

fn require_extension(path: &Path, expected: &str) -> Result<(), String> {
    let actual = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!("Export path must use the .{expected} extension."));
    }
    Ok(())
}

fn option_number(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "N/A".into())
}
fn escape_markdown_cell(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}
fn typst_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len() + 2);
    escaped.push('"');
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => {}
            '\t' => escaped.push_str("\\t"),
            value if value.is_control() => escaped.push(' '),
            value => escaped.push(value),
        }
    }
    escaped.push('"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    #[test]
    fn markdown_roundtrip_restores_identical_state() {
        let session = SessionState::empty();
        let dir =
            std::env::temp_dir().join(format!("agentic-council-report-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.md");
        export_markdown(&session, &path).unwrap();
        let restored = import_markdown(&path).unwrap();
        assert_eq!(restored.id, session.id);
        assert_eq!(restored.agents.len(), session.agents.len());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pdf_export_writes_a_valid_pdf_header() {
        let mut session = SessionState::empty();
        session.objective = "Verify [brackets], {braces}, (parentheses), *stars*, _underscores_, `ticks`, #hashes, \\\\slashes, \"quotes\", $math, @refs, <labels>, and emoji 🧪.\nNo delimiter may escape.".into();
        session.aspects.push(Aspect {
            id: "edge-case".into(),
            name: "Syntax [safety] *test*".into(),
            description: "Compile every delimiter: [ ] { } ( ) * _ ` # $ @ < > \\\\".into(),
            weight: 1.0,
        });
        session.rounds.push(RoundRecord {
            index: 1,
            started_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            responses: vec![AgentResponse {
                agent_id: "member-1".into(),
                content: "A response with unmatched [ * _ ` { ( and a quoted \"value\".".into(),
                status: AgentStatus::Complete,
                error: None,
                input_tokens: Some(10),
                output_tokens: Some(20),
                latency_ms: 42,
            }],
            friction: vec![FrictionItem {
                id: "friction".into(),
                kind: FrictionKind::Omission,
                agent_ids: vec!["member-1".into()],
                aspect_id: Some("edge-case".into()),
                explanation: "Missing ] } ) * _ ` delimiters.".into(),
                challenge: "Can #Typst safely render $all @of <these>?".into(),
            }],
            scores: vec![ScoreCell {
                agent_id: "member-1".into(),
                aspect_id: "edge-case".into(),
                median: 8.5,
                votes: vec![VoteDetail {
                    voter_alias: "Councilor 2 · model/[safe]".into(),
                    score: 8.5,
                    outlier: false,
                }],
            }],
            user_argument: None,
            semantic_similarity: Some(0.0),
            consensus: Some(100.0),
        });
        session.final_synthesis = Some("Final [safe] synthesis by Parsa's app.".into());
        let dir =
            std::env::temp_dir().join(format!("agentic-council-pdf-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("session.pdf");
        let result = export_pdf(&session, &path).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
        assert_eq!(result.bytes, bytes.len() as u64);
        if let Ok(qa_path) = std::env::var("AGENTIC_COUNCIL_PDF_QA_PATH") {
            let qa_path = std::path::PathBuf::from(qa_path);
            fs::create_dir_all(qa_path.parent().unwrap()).unwrap();
            fs::copy(&path, qa_path).unwrap();
        }
        let _ = fs::remove_dir_all(dir);
    }
}

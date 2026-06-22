use codex_app_server_protocol::ThreadDecision;
use codex_app_server_protocol::ThreadDecisionStatus;
use codex_app_server_protocol::ThreadDecisionUrgency;
use codex_app_server_protocol::ThreadRequirement;
use codex_app_server_protocol::ThreadRequirementStatus;
use ratatui::style::Stylize;
use ratatui::text::Line;

use crate::keymap::PagerKeymap;
use crate::pager_overlay::Overlay;

const OVERLAY_TITLE: &str = "R E Q U I R E M E N T";
const NO_OBJECTIVE: &str = "No requirement objective set";
const NO_SUMMARY: &str = "No outcome summary available yet";
const NO_DECISIONS: &str = "No decisions recorded yet";

pub(crate) fn requirement_overlay(requirement: &ThreadRequirement, keymap: PagerKeymap) -> Overlay {
    Overlay::new_static_with_lines(
        requirement_lines(requirement),
        OVERLAY_TITLE.to_string(),
        keymap,
    )
}

fn requirement_lines(requirement: &ThreadRequirement) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    push_section(&mut lines, "Objective");
    push_text_or_fallback(&mut lines, requirement.objective.as_deref(), NO_OBJECTIVE);
    lines.push(Line::from(""));

    push_section(&mut lines, "Status");
    lines.push(Line::from(requirement_status_label(requirement.status)));
    lines.push(Line::from(""));

    push_section(&mut lines, "Summary");
    push_text_or_fallback(&mut lines, Some(requirement.summary.as_str()), NO_SUMMARY);
    lines.push(Line::from(""));

    push_section(&mut lines, "Decisions");
    if requirement.decisions.is_empty() {
        lines.push(Line::from(NO_DECISIONS));
    } else {
        let mut decisions = requirement.decisions.iter().collect::<Vec<_>>();
        decisions.sort_by_key(|decision| {
            (
                urgency_sort_key(decision.urgency),
                decision_status_sort_key(decision.status),
            )
        });
        for decision in decisions {
            push_decision(&mut lines, decision);
        }
    }

    lines
}

fn push_section(lines: &mut Vec<Line<'static>>, label: &'static str) {
    lines.push(Line::from(label).bold());
}

fn push_text_or_fallback(
    lines: &mut Vec<Line<'static>>,
    value: Option<&str>,
    fallback: &'static str,
) {
    let value = value.map(str::trim).filter(|value| !value.is_empty());
    match value {
        Some(value) => push_multiline(lines, value),
        None => lines.push(Line::from(fallback).italic()),
    }
}

fn push_multiline(lines: &mut Vec<Line<'static>>, value: &str) {
    lines.extend(value.lines().map(|line| Line::from(line.to_string())));
}

fn push_decision(lines: &mut Vec<Line<'static>>, decision: &ThreadDecision) {
    lines.push(Line::from(vec![
        "- ".into(),
        urgency_label(decision.urgency).bold(),
        " / ".dim(),
        decision_status_label(decision.status).into(),
        ": ".dim(),
        decision.title.clone().into(),
    ]));
    push_indented_field(lines, "Description", decision.description.as_str());
    if let Some(resolution) = decision.resolution.as_deref() {
        push_indented_field(lines, "Resolution", resolution);
    }
    if let Some(recommendation) = decision.recommendation.as_deref() {
        push_indented_field(lines, "Recommendation", recommendation);
    }
    if !decision.options.is_empty() {
        let options = decision
            .options
            .iter()
            .map(|option| match option.description.as_deref() {
                Some(description) if !description.trim().is_empty() => {
                    format!("{} ({})", option.label, description.trim())
                }
                _ => option.label.clone(),
            })
            .collect::<Vec<_>>()
            .join("; ");
        push_indented_field(lines, "Options", &options);
    }
}

fn push_indented_field(lines: &mut Vec<Line<'static>>, label: &'static str, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    for (index, line) in value.lines().enumerate() {
        if index == 0 {
            lines.push(Line::from(format!("  {label}: {}", line.trim())));
        } else {
            lines.push(Line::from(format!("  {}", line.trim())));
        }
    }
}

fn requirement_status_label(status: ThreadRequirementStatus) -> &'static str {
    match status {
        ThreadRequirementStatus::NotStarted => "not started",
        ThreadRequirementStatus::Running => "running",
        ThreadRequirementStatus::WaitingOnDecision => "waiting on decision",
        ThreadRequirementStatus::Complete => "complete",
        ThreadRequirementStatus::Failed => "failed",
        ThreadRequirementStatus::Unknown => "unknown",
    }
}

fn urgency_label(urgency: ThreadDecisionUrgency) -> &'static str {
    match urgency {
        ThreadDecisionUrgency::Immediate => "immediate",
        ThreadDecisionUrgency::Deferred => "deferred",
    }
}

fn decision_status_label(status: ThreadDecisionStatus) -> &'static str {
    match status {
        ThreadDecisionStatus::Pending => "pending",
        ThreadDecisionStatus::Resolved => "resolved",
        ThreadDecisionStatus::Deferred => "deferred",
    }
}

fn urgency_sort_key(urgency: ThreadDecisionUrgency) -> u8 {
    match urgency {
        ThreadDecisionUrgency::Immediate => 0,
        ThreadDecisionUrgency::Deferred => 1,
    }
}

fn decision_status_sort_key(status: ThreadDecisionStatus) -> u8 {
    match status {
        ThreadDecisionStatus::Pending => 0,
        ThreadDecisionStatus::Deferred => 1,
        ThreadDecisionStatus::Resolved => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use codex_app_server_protocol::ThreadDecisionOption;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn default_pager_keymap() -> PagerKeymap {
        crate::keymap::RuntimeKeymap::defaults().pager
    }

    fn assert_requirement_snapshot(name: &str, requirement: &ThreadRequirement) {
        let mut overlay = match requirement_overlay(requirement, default_pager_keymap()) {
            Overlay::Static(overlay) => overlay,
            Overlay::Transcript(_) => unreachable!("requirement overlay must be static"),
        };
        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame.area(), frame.buffer_mut()))
            .expect("draw");
        insta::assert_snapshot!(name, terminal.backend());
    }

    #[test]
    fn requirement_overlay_renders_outcome() {
        let requirement = ThreadRequirement {
            thread_id: "thread-1".to_string(),
            objective: Some("Ship the requirements outcome view.".to_string()),
            status: ThreadRequirementStatus::WaitingOnDecision,
            summary: "The backend exposes an outcome summary ready for TUI display.".to_string(),
            decisions: vec![ThreadDecision {
                id: "decision-1".to_string(),
                thread_id: "thread-1".to_string(),
                title: "Pick the first entry point".to_string(),
                description: "Use a slash command for discoverability.".to_string(),
                urgency: ThreadDecisionUrgency::Immediate,
                status: ThreadDecisionStatus::Pending,
                options: vec![
                    ThreadDecisionOption {
                        id: "opt-1".to_string(),
                        label: "/requirements".to_string(),
                        description: Some("Open the full requirement outcome.".to_string()),
                    },
                    ThreadDecisionOption {
                        id: "opt-2".to_string(),
                        label: "keyboard shortcut".to_string(),
                        description: None,
                    },
                ],
                recommendation: Some("Start with the slash command.".to_string()),
                source_turn_id: Some("turn-1".to_string()),
                resolved_at: None,
                resolution: None,
                selected_option_id: None,
            }],
            updated_at: Some(1_776_272_400),
        };

        assert_requirement_snapshot("requirement_overlay_outcome", &requirement);
    }

    #[test]
    fn requirement_overlay_renders_empty_state() {
        let requirement = ThreadRequirement {
            thread_id: "thread-1".to_string(),
            objective: None,
            status: ThreadRequirementStatus::NotStarted,
            summary: String::new(),
            decisions: Vec::new(),
            updated_at: None,
        };

        assert_requirement_snapshot("requirement_overlay_empty_state", &requirement);
    }
}

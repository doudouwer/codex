use super::*;
use pretty_assertions::assert_eq;

fn completed_turn(id: &str, items: Vec<ThreadItem>) -> Turn {
    Turn {
        id: id.to_string(),
        items,
        items_view: TurnItemsView::Full,
        status: TurnStatus::Completed,
        error: None,
        started_at: None,
        completed_at: None,
        duration_ms: None,
    }
}

#[test]
fn derives_requirement_from_latest_agent_message_and_plan_lines() {
    let turns = vec![
        completed_turn(
            "turn-1",
            vec![ThreadItem::AgentMessage {
                id: "agent-1".to_string(),
                text: "Initial summary".to_string(),
                phase: None,
                memory_citation: None,
            }],
        ),
        completed_turn(
            "turn-2",
            vec![
                ThreadItem::Plan {
                    id: "plan-1".to_string(),
                    text: "- Choose database\n\n2) Pick deployment target".to_string(),
                },
                ThreadItem::AgentMessage {
                    id: "agent-2".to_string(),
                    text: "Latest summary".to_string(),
                    phase: None,
                    memory_citation: None,
                },
            ],
        ),
    ];

    let requirement = build_thread_requirement(
        RequirementSource {
            thread_id: "thr_123".to_string(),
            status: ThreadStatus::NotLoaded,
            turns,
            updated_at: Some(1_700_000_000),
        },
        Some("Ship requirement view".to_string()),
    );

    assert_eq!(
        requirement,
        ThreadRequirement {
            thread_id: "thr_123".to_string(),
            objective: Some("Ship requirement view".to_string()),
            status: ThreadRequirementStatus::WaitingOnDecision,
            summary: "Latest summary".to_string(),
            decisions: vec![
                ThreadDecision {
                    id: "plan-1:0".to_string(),
                    thread_id: "thr_123".to_string(),
                    title: "Choose database".to_string(),
                    description: String::new(),
                    urgency: ThreadDecisionUrgency::Deferred,
                    status: ThreadDecisionStatus::Pending,
                    options: Vec::new(),
                    recommendation: None,
                    source_turn_id: Some("turn-2".to_string()),
                    resolved_at: None,
                    resolution: None,
                    selected_option_id: None,
                },
                ThreadDecision {
                    id: "plan-1:1".to_string(),
                    thread_id: "thr_123".to_string(),
                    title: "Pick deployment target".to_string(),
                    description: String::new(),
                    urgency: ThreadDecisionUrgency::Deferred,
                    status: ThreadDecisionStatus::Pending,
                    options: Vec::new(),
                    recommendation: None,
                    source_turn_id: Some("turn-2".to_string()),
                    resolved_at: None,
                    resolution: None,
                    selected_option_id: None,
                },
            ],
            updated_at: Some(1_700_000_000),
        }
    );
}

#[test]
fn returns_empty_decisions_when_no_plan_item_exists() {
    let turns = vec![completed_turn(
        "turn-1",
        vec![ThreadItem::AgentMessage {
            id: "agent-1".to_string(),
            text: "Done".to_string(),
            phase: None,
            memory_citation: None,
        }],
    )];

    let requirement = build_thread_requirement(
        RequirementSource {
            thread_id: "thr_123".to_string(),
            status: ThreadStatus::Idle,
            turns,
            updated_at: None,
        },
        None,
    );

    assert_eq!(
        requirement,
        ThreadRequirement {
            thread_id: "thr_123".to_string(),
            objective: None,
            status: ThreadRequirementStatus::Complete,
            summary: "Done".to_string(),
            decisions: Vec::new(),
            updated_at: None,
        }
    );
}

#[test]
fn requirement_summary_hides_fenced_code_blocks() {
    let turns = vec![completed_turn(
        "turn-1",
        vec![ThreadItem::AgentMessage {
            id: "agent-1".to_string(),
            text: "The requirement is implemented.\n\n```rust\nfn internal_detail() {}\n```\n\nTests passed."
                .to_string(),
            phase: None,
            memory_citation: None,
        }],
    )];

    let requirement = build_thread_requirement(
        RequirementSource {
            thread_id: "thr_123".to_string(),
            status: ThreadStatus::Idle,
            turns,
            updated_at: None,
        },
        None,
    );

    assert_eq!(
        requirement.summary,
        "The requirement is implemented.\n\nTests passed."
    );
}

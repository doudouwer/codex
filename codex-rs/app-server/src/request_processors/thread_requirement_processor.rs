use super::*;
use codex_goal_extension::GoalService;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;

#[derive(Clone)]
pub(crate) struct ThreadRequirementRequestProcessor {
    thread_manager: Arc<ThreadManager>,
    config: Arc<Config>,
    thread_store: Arc<dyn ThreadStore>,
    state_db: Option<StateDbHandle>,
    goal_service: Arc<GoalService>,
}

impl ThreadRequirementRequestProcessor {
    pub(crate) fn new(
        thread_manager: Arc<ThreadManager>,
        config: Arc<Config>,
        thread_store: Arc<dyn ThreadStore>,
        state_db: Option<StateDbHandle>,
        goal_service: Arc<GoalService>,
    ) -> Self {
        Self {
            thread_manager,
            config,
            thread_store,
            state_db,
            goal_service,
        }
    }

    pub(crate) async fn thread_requirement_read(
        &self,
        params: ThreadRequirementReadParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread_id = parse_thread_id(params.thread_id.as_str())?;
        let source = self.load_requirement_source(thread_id).await?;
        let objective = self.load_goal_objective(thread_id).await;
        let requirement = build_thread_requirement(source, objective);
        Ok(Some(ThreadRequirementReadResponse { requirement }.into()))
    }

    pub(crate) async fn thread_decision_list(
        &self,
        params: ThreadDecisionListParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread_id = parse_thread_id(params.thread_id.as_str())?;
        let source = self.load_requirement_source(thread_id).await?;
        let mut decisions = derive_decisions(&source.thread_id, source.turns.as_slice());
        if let Some(status) = params.status {
            decisions.retain(|decision| decision.status == status);
        }
        if let Some(urgency) = params.urgency {
            decisions.retain(|decision| decision.urgency == urgency);
        }
        Ok(Some(ThreadDecisionListResponse { data: decisions }.into()))
    }

    pub(crate) async fn thread_decision_resolve(
        &self,
        params: ThreadDecisionResolveParams,
    ) -> Result<Option<ClientResponsePayload>, JSONRPCErrorError> {
        let thread_id = parse_thread_id(params.thread_id.as_str())?;
        let decision_id = params.decision_id.trim();
        if decision_id.is_empty() {
            return Err(invalid_request("decisionId must not be empty"));
        }

        let status = if params.defer {
            ThreadDecisionStatus::Deferred
        } else {
            ThreadDecisionStatus::Resolved
        };
        let resolved_at = if params.defer {
            None
        } else {
            current_unix_timestamp()
        };
        let decision = ThreadDecision {
            id: decision_id.to_string(),
            thread_id: thread_id.to_string(),
            title: decision_id.to_string(),
            description: params.resolution.clone().unwrap_or_default(),
            urgency: ThreadDecisionUrgency::Deferred,
            status,
            options: Vec::new(),
            recommendation: None,
            source_turn_id: None,
            resolved_at,
            resolution: params.resolution,
            selected_option_id: params.selected_option_id,
        };
        Ok(Some(ThreadDecisionResolveResponse { decision }.into()))
    }

    async fn load_requirement_source(
        &self,
        thread_id: ThreadId,
    ) -> Result<RequirementSource, JSONRPCErrorError> {
        match self
            .thread_store
            .read_thread(StoreReadThreadParams {
                thread_id,
                include_archived: true,
                include_history: true,
            })
            .await
        {
            Ok(stored_thread) => {
                let updated_at = Some(stored_thread.updated_at.timestamp());
                let turns = stored_thread
                    .history
                    .map(|history| build_turns_from_rollout_items(&history.items))
                    .unwrap_or_default();
                return Ok(RequirementSource {
                    thread_id: thread_id.to_string(),
                    status: ThreadStatus::NotLoaded,
                    turns,
                    updated_at,
                });
            }
            Err(ThreadStoreError::InvalidRequest { message })
                if message == format!("no rollout found for thread id {thread_id}") => {}
            Err(ThreadStoreError::ThreadNotFound {
                thread_id: missing_thread_id,
            }) if missing_thread_id == thread_id => {}
            Err(ThreadStoreError::InvalidRequest { message }) => {
                return Err(invalid_request(message));
            }
            Err(ThreadStoreError::Unsupported { operation }) => {
                return Err(super::thread_processor::unsupported_thread_store_operation(
                    operation,
                ));
            }
            Err(err) => {
                return Err(internal_error(format!(
                    "failed to read thread requirement: {err}"
                )));
            }
        }

        let thread = self
            .thread_manager
            .get_thread(thread_id)
            .await
            .map_err(|_| invalid_request(format!("thread not found: {thread_id}")))?;
        let status = match thread.agent_status().await {
            AgentStatus::Running | AgentStatus::PendingInit => ThreadStatus::Active {
                active_flags: Vec::new(),
            },
            AgentStatus::Errored(_) => ThreadStatus::SystemError,
            AgentStatus::Interrupted | AgentStatus::Completed(_) | AgentStatus::Shutdown => {
                ThreadStatus::Idle
            }
            AgentStatus::NotFound => ThreadStatus::NotLoaded,
        };
        let config_snapshot = thread.config_snapshot().await;
        let turns = if config_snapshot.ephemeral {
            Vec::new()
        } else {
            match thread.load_history(/*include_archived*/ true).await {
                Ok(history) => build_turns_from_rollout_items(&history.items),
                Err(err) => {
                    warn!("failed to load live thread history for requirement surface: {err}");
                    Vec::new()
                }
            }
        };
        Ok(RequirementSource {
            thread_id: thread_id.to_string(),
            status,
            turns,
            updated_at: None,
        })
    }

    async fn load_goal_objective(&self, thread_id: ThreadId) -> Option<String> {
        if !self.config.features.enabled(Feature::Goals) {
            return None;
        }

        let state_db = match self.thread_manager.get_thread(thread_id).await {
            Ok(thread) => thread.state_db().or_else(|| self.state_db.clone()),
            Err(_) => self.state_db.clone(),
        }?;

        match self
            .goal_service
            .get_thread_goal(&state_db, thread_id)
            .await
        {
            Ok(goal) => goal.map(|goal| goal.objective),
            Err(err) => {
                warn!("failed to load thread goal for requirement surface: {err}");
                None
            }
        }
    }
}

struct RequirementSource {
    thread_id: String,
    status: ThreadStatus,
    turns: Vec<Turn>,
    updated_at: Option<i64>,
}

fn build_thread_requirement(
    source: RequirementSource,
    objective: Option<String>,
) -> ThreadRequirement {
    let decisions = derive_decisions(&source.thread_id, source.turns.as_slice());
    let summary = latest_outcome_summary(source.turns.as_slice()).unwrap_or_default();
    let status = derive_requirement_status(&source.status, source.turns.as_slice(), &decisions);
    ThreadRequirement {
        thread_id: source.thread_id,
        objective,
        status,
        summary,
        decisions,
        updated_at: source.updated_at,
    }
}

fn build_turns_from_rollout_items(items: &[RolloutItem]) -> Vec<Turn> {
    let mut builder = ThreadHistoryBuilder::new();
    for item in items {
        builder.handle_rollout_item(item);
    }
    builder.finish()
}

fn latest_agent_message(turns: &[Turn]) -> Option<String> {
    turns
        .iter()
        .rev()
        .flat_map(|turn| turn.items.iter().rev())
        .find_map(|item| match item {
            ThreadItem::AgentMessage { text, .. } if !text.trim().is_empty() => Some(text.clone()),
            _ => None,
        })
}

fn latest_outcome_summary(turns: &[Turn]) -> Option<String> {
    latest_agent_message(turns)
        .map(|message| strip_fenced_code_blocks(&message))
        .filter(|message| !message.trim().is_empty())
}

fn strip_fenced_code_blocks(message: &str) -> String {
    let mut visible_lines = Vec::new();
    let mut in_fence = false;
    for line in message.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence {
            visible_lines.push(line);
        }
    }
    let mut compacted_lines = Vec::new();
    for line in visible_lines {
        let is_blank = line.trim().is_empty();
        let previous_is_blank = compacted_lines
            .last()
            .is_some_and(|previous: &&str| previous.trim().is_empty());
        if is_blank && previous_is_blank {
            continue;
        }
        compacted_lines.push(line);
    }
    compacted_lines.join("\n").trim().to_string()
}

fn derive_requirement_status(
    thread_status: &ThreadStatus,
    turns: &[Turn],
    decisions: &[ThreadDecision],
) -> ThreadRequirementStatus {
    match thread_status {
        ThreadStatus::SystemError => ThreadRequirementStatus::Failed,
        ThreadStatus::Active { .. } => ThreadRequirementStatus::Running,
        ThreadStatus::NotLoaded | ThreadStatus::Idle => {
            if decisions
                .iter()
                .any(|decision| decision.status == ThreadDecisionStatus::Pending)
            {
                ThreadRequirementStatus::WaitingOnDecision
            } else if turns.is_empty() {
                ThreadRequirementStatus::NotStarted
            } else if latest_outcome_summary(turns).is_some() {
                ThreadRequirementStatus::Complete
            } else {
                ThreadRequirementStatus::Unknown
            }
        }
    }
}

fn derive_decisions(thread_id: &str, turns: &[Turn]) -> Vec<ThreadDecision> {
    let mut decisions = Vec::new();
    for turn in turns {
        for item in &turn.items {
            let ThreadItem::Plan { id, text } = item else {
                continue;
            };
            for (line_index, title) in plan_decision_titles(text).into_iter().enumerate() {
                decisions.push(ThreadDecision {
                    id: format!("{id}:{line_index}"),
                    thread_id: thread_id.to_string(),
                    title,
                    description: String::new(),
                    urgency: ThreadDecisionUrgency::Deferred,
                    status: ThreadDecisionStatus::Pending,
                    options: Vec::new(),
                    recommendation: None,
                    source_turn_id: Some(turn.id.clone()),
                    resolved_at: None,
                    resolution: None,
                    selected_option_id: None,
                });
            }
        }
    }
    decisions
}

fn plan_decision_titles(text: &str) -> Vec<String> {
    text.lines()
        .filter_map(|line| {
            let title = clean_plan_line(line);
            (!title.is_empty()).then_some(title)
        })
        .collect()
}

fn clean_plan_line(line: &str) -> String {
    let mut value = line.trim();
    value = value
        .trim_start_matches(['-', '*', '+'])
        .trim_start_matches(char::is_whitespace);
    if let Some(stripped) = value.strip_prefix("[ ]") {
        value = stripped.trim_start();
    } else if let Some(stripped) = value.strip_prefix("[x]") {
        value = stripped.trim_start();
    }

    let mut chars = value.char_indices();
    let mut digit_end = 0;
    for (index, ch) in &mut chars {
        if ch.is_ascii_digit() {
            digit_end = index + ch.len_utf8();
        } else if ch == '.' || ch == ')' {
            value = value[digit_end..]
                .trim_start_matches(['.', ')'])
                .trim_start();
            break;
        } else {
            break;
        }
    }
    value.to_string()
}

fn parse_thread_id(thread_id: &str) -> Result<ThreadId, JSONRPCErrorError> {
    ThreadId::from_string(thread_id)
        .map_err(|err| invalid_request(format!("invalid thread id: {err}")))
}

fn current_unix_timestamp() -> Option<i64> {
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    Some(duration.as_secs() as i64)
}

#[cfg(test)]
#[path = "thread_requirement_processor_tests.rs"]
mod tests;

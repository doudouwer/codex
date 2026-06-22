use axum::response::Html;

pub(crate) async fn requirement_page() -> Html<&'static str> {
    Html(REQUIREMENT_PAGE_HTML)
}

const REQUIREMENT_PAGE_HTML: &str = r##"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Codex Requirement View</title>
  <style>
    :root { color-scheme: light dark; font-family: Inter, ui-sans-serif, system-ui, sans-serif; }
    body { margin: 0; background: #0f1115; color: #edf0f7; }
    main { max-width: 1120px; margin: 0 auto; padding: 32px; }
    header { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 24px; }
    h1 { font-size: 28px; margin: 0 0 8px; }
    h2 { font-size: 15px; letter-spacing: .08em; text-transform: uppercase; color: #9da7bb; margin: 0 0 12px; }
    p { color: #c8cfdd; line-height: 1.5; }
    .status { color: #9da7bb; font-size: 13px; }
    .panel { background: #171a21; border: 1px solid #2a3040; border-radius: 16px; padding: 20px; box-shadow: 0 16px 40px rgb(0 0 0 / 20%); }
    .layout { display: grid; grid-template-columns: minmax(0, 1fr) 360px; gap: 20px; }
    .stack { display: grid; gap: 20px; }
    label { display: block; color: #9da7bb; font-size: 13px; margin-bottom: 8px; }
    input, select, textarea { width: 100%; box-sizing: border-box; border: 1px solid #3a4257; border-radius: 10px; background: #0f1115; color: #edf0f7; padding: 10px 12px; }
    button { border: 0; border-radius: 10px; background: #6ea8fe; color: #07111f; font-weight: 700; padding: 10px 14px; cursor: pointer; }
    button.secondary { background: #2a3040; color: #edf0f7; }
    button:disabled { opacity: .55; cursor: default; }
    .controls { display: flex; gap: 10px; align-items: end; }
    .controls > div { flex: 1; }
    .summary { white-space: pre-wrap; }
    .badge { display: inline-flex; align-items: center; border-radius: 999px; padding: 4px 10px; background: #253149; color: #b8cdfd; font-size: 12px; font-weight: 700; }
    .decision { border-top: 1px solid #2a3040; padding-top: 16px; margin-top: 16px; }
    .decision:first-child { border-top: 0; margin-top: 0; padding-top: 0; }
    .decision h3 { margin: 8px 0; font-size: 16px; }
    .muted { color: #8d96a9; }
    .error { color: #ff9d9d; }
    .thread { display: grid; gap: 8px; margin-top: 10px; }
    .thread button { text-align: left; background: #202637; color: #edf0f7; }
    @media (max-width: 860px) { main { padding: 20px; } .layout { grid-template-columns: 1fr; } header { display: block; } }
  </style>
</head>
<body>
  <main>
    <header>
      <div>
        <h1>Codex Requirement View</h1>
        <p>Outcome-focused status and decisions for a Codex thread. Implementation details stay out of this page.</p>
      </div>
      <div class="status" id="connection">Connecting...</div>
    </header>

    <section class="panel controls">
      <div>
        <label for="thread-id">Thread ID</label>
        <input id="thread-id" autocomplete="off" placeholder="Paste a thread id or pick a recent thread" />
      </div>
      <button id="load">Load requirement</button>
    </section>

    <div class="layout" style="margin-top: 20px;">
      <section class="stack">
        <article class="panel">
          <h2>Objective</h2>
          <p id="objective" class="muted">No requirement loaded.</p>
        </article>
        <article class="panel">
          <h2>Status</h2>
          <span id="requirement-status" class="badge">unknown</span>
        </article>
        <article class="panel">
          <h2>Summary</h2>
          <p id="summary" class="summary muted">No outcome summary available yet.</p>
        </article>
        <article class="panel">
          <h2>Decisions</h2>
          <div id="decisions" class="muted">No decisions recorded yet.</div>
        </article>
      </section>

      <aside class="panel">
        <h2>Recent threads</h2>
        <div id="threads" class="thread muted">Loading recent threads...</div>
      </aside>
    </div>
  </main>

  <script>
    const state = { ws: null, nextId: 1, pending: new Map(), requirement: null };
    const $ = (id) => document.getElementById(id);
    const connection = $("connection");
    const threadInput = $("thread-id");
    const loadButton = $("load");

    function setConnection(text, error = false) {
      connection.textContent = text;
      connection.className = error ? "status error" : "status";
    }

    function request(method, params) {
      const id = state.nextId++;
      const payload = params === undefined ? { id, method } : { id, method, params };
      state.ws.send(JSON.stringify(payload));
      return new Promise((resolve, reject) => {
        state.pending.set(id, { resolve, reject });
        setTimeout(() => {
          if (state.pending.delete(id)) reject(new Error(`${method} timed out`));
        }, 30000);
      });
    }

    function connect() {
      const scheme = location.protocol === "https:" ? "wss:" : "ws:";
      state.ws = new WebSocket(`${scheme}//${location.host}/ws`);
      state.ws.addEventListener("open", async () => {
        try {
          setConnection("Connected");
          await request("initialize", {
            clientInfo: { name: "codex_requirement_page", title: "Codex Requirement Page", version: "0.1.0" }
          });
          await loadThreads();
          const initialThread = new URLSearchParams(location.search).get("threadId");
          if (initialThread) {
            threadInput.value = initialThread;
            await loadRequirement(initialThread);
          }
        } catch (error) {
          setConnection(error.message, true);
        }
      });
      state.ws.addEventListener("message", (event) => {
        const message = JSON.parse(event.data);
        if (!("id" in message)) return;
        const pending = state.pending.get(message.id);
        if (!pending) return;
        state.pending.delete(message.id);
        if (message.error) pending.reject(new Error(message.error.message || "Request failed"));
        else pending.resolve(message.result);
      });
      state.ws.addEventListener("close", () => setConnection("Disconnected", true));
      state.ws.addEventListener("error", () => setConnection("Connection error", true));
    }

    async function loadThreads() {
      const result = await request("thread/list", { limit: 12 });
      const threads = $("threads");
      threads.innerHTML = "";
      if (!result.data.length) {
        threads.textContent = "No recent threads found.";
        return;
      }
      for (const thread of result.data) {
        const button = document.createElement("button");
        button.type = "button";
        button.innerHTML = `<strong>${escapeHtml(thread.name || thread.preview || thread.id)}</strong><br><span class="muted">${escapeHtml(thread.id)}</span>`;
        button.addEventListener("click", () => {
          threadInput.value = thread.id;
          loadRequirement(thread.id);
        });
        threads.appendChild(button);
      }
    }

    async function loadRequirement(threadId) {
      if (!threadId.trim()) return;
      loadButton.disabled = true;
      try {
        const result = await request("thread/requirement/read", { threadId: threadId.trim() });
        state.requirement = result.requirement;
        renderRequirement(result.requirement);
        history.replaceState(null, "", `?threadId=${encodeURIComponent(threadId.trim())}`);
      } catch (error) {
        $("summary").textContent = error.message;
        $("summary").className = "summary error";
      } finally {
        loadButton.disabled = false;
      }
    }

    function renderRequirement(requirement) {
      $("objective").textContent = requirement.objective || "No requirement objective set.";
      $("objective").className = requirement.objective ? "" : "muted";
      $("requirement-status").textContent = label(requirement.status);
      $("summary").textContent = requirement.summary || "No outcome summary available yet.";
      $("summary").className = requirement.summary ? "summary" : "summary muted";
      renderDecisions(requirement.decisions || []);
    }

    function renderDecisions(decisions) {
      const root = $("decisions");
      root.innerHTML = "";
      if (!decisions.length) {
        root.textContent = "No decisions recorded yet.";
        root.className = "muted";
        return;
      }
      root.className = "";
      decisions
        .slice()
        .sort((a, b) => sortKey(a).localeCompare(sortKey(b)))
        .forEach((decision) => root.appendChild(decisionElement(decision)));
    }

    function decisionElement(decision) {
      const item = document.createElement("div");
      item.className = "decision";
      item.innerHTML = `
        <span class="badge">${label(decision.urgency)} / ${label(decision.status)}</span>
        <h3>${escapeHtml(decision.title)}</h3>
        ${decision.description ? `<p>${escapeHtml(decision.description)}</p>` : ""}
        ${decision.recommendation ? `<p><strong>Recommendation:</strong> ${escapeHtml(decision.recommendation)}</p>` : ""}
      `;
      if (decision.status === "pending") {
        const resolve = document.createElement("button");
        resolve.textContent = "Resolve";
        resolve.addEventListener("click", () => resolveDecision(decision, false));
        const defer = document.createElement("button");
        defer.textContent = "Defer";
        defer.className = "secondary";
        defer.addEventListener("click", () => resolveDecision(decision, true));
        item.append(resolve, " ", defer);
      }
      return item;
    }

    async function resolveDecision(decision, defer) {
      const resolution = prompt(defer ? "Why defer this decision?" : "Resolution", decision.resolution || "");
      if (resolution === null || !state.requirement) return;
      const result = await request("thread/decision/resolve", {
        threadId: state.requirement.threadId,
        decisionId: decision.id,
        resolution,
        defer
      });
      state.requirement.decisions = state.requirement.decisions.map((item) =>
        item.id === decision.id ? result.decision : item
      );
      renderRequirement(state.requirement);
    }

    function label(value) {
      return String(value || "unknown").replace(/[A-Z]/g, (match) => ` ${match.toLowerCase()}`).trim();
    }

    function sortKey(decision) {
      const urgency = decision.urgency === "immediate" ? "0" : "1";
      const status = decision.status === "pending" ? "0" : decision.status === "deferred" ? "1" : "2";
      return `${urgency}:${status}:${decision.title}`;
    }

    function escapeHtml(value) {
      return String(value).replace(/[&<>"']/g, (char) => ({
        "&": "&amp;", "<": "&lt;", ">": "&gt;", "\"": "&quot;", "'": "&#39;"
      })[char]);
    }

    loadButton.addEventListener("click", () => loadRequirement(threadInput.value));
    threadInput.addEventListener("keydown", (event) => {
      if (event.key === "Enter") loadRequirement(threadInput.value);
    });
    connect();
  </script>
</body>
</html>"##;

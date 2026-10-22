//! An agent run's output, read line by line (R5, `docs/design/chat-composer.md` › The parsers).
//! - **Claude Code** (`stream-json`): `system`/`init` lists the MCP servers and their status;
//!   `assistant` carries text and tool calls; the last event, `result`, has the reply, `is_error`,
//!   `total_cost_usd` and `usage`.
//! - **Codex** (`exec --json`): `item.completed` with an `agent_message` is the reply so far;
//!   `turn.completed` carries `usage`; `turn.failed` and `error` are failures.
//! - A line that isn't JSON, or an event not listed here, is skipped.
//! - Tokens and cost are only ever what the CLI reported: nothing is estimated.

use serde_json::Value;

use super::Output;

/// What a line of output says.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// The run began; whether Scrim's MCP server was reached, when the CLI says.
    Started { scrim_connected: Option<bool> },
    /// The agent called a tool (its name, as the CLI gives it).
    Tool(String),
    /// The agent's reply so far (Codex sends it before the turn ends).
    Reply(String),
    /// Tokens the CLI reported.
    Usage { input: Option<u64>, output: Option<u64> },
    /// The cost the CLI reported, in US dollars.
    Cost(f64),
    /// The run finished: whether the CLI calls it an error.
    Finished { error: bool },
    /// The CLI reported a failure, in its words.
    Failed(String),
}

/// Read one line of `output`'s kind.
pub fn parse(output: Output, line: &str) -> Vec<Event> {
    let Ok(v) = serde_json::from_str::<Value>(line.trim()) else { return Vec::new() };
    match output {
        Output::ClaudeStreamJson => claude(&v),
        Output::CodexJsonl => codex(&v),
    }
}

fn text(v: &Value) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(str::to_string)
}

fn usage(u: &Value) -> Option<Event> {
    let (input, output) = (u["input_tokens"].as_u64(), u["output_tokens"].as_u64());
    (input.is_some() || output.is_some()).then_some(Event::Usage { input, output })
}

fn claude(v: &Value) -> Vec<Event> {
    match v["type"].as_str() {
        Some("system") if v["subtype"] == "init" => {
            let scrim = v["mcp_servers"].as_array().and_then(|a| a.iter().find(|s| s["name"] == "scrim"));
            vec![Event::Started { scrim_connected: scrim.map(|s| s["status"] == "connected") }]
        }
        Some("assistant") => v["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|c| (c["type"] == "tool_use").then(|| c["name"].as_str().map(|n| Event::Tool(n.to_string()))).flatten())
            .collect(),
        Some("result") => {
            let error = v["is_error"].as_bool().unwrap_or(false) || v["subtype"].as_str().is_some_and(|s| s != "success");
            let mut out = Vec::new();
            if let Some(r) = text(&v["result"]) {
                out.push(Event::Reply(r));
            }
            if let Some(u) = usage(&v["usage"]) {
                out.push(u);
            }
            if let Some(c) = v["total_cost_usd"].as_f64().filter(|c| c.is_finite() && *c >= 0.0) {
                out.push(Event::Cost(c));
            }
            out.push(Event::Finished { error });
            out
        }
        _ => Vec::new(),
    }
}

fn codex(v: &Value) -> Vec<Event> {
    match v["type"].as_str() {
        Some("thread.started") => vec![Event::Started { scrim_connected: None }],
        Some("item.completed") => {
            let item = &v["item"];
            match item["type"].as_str() {
                Some("agent_message") => text(&item["text"]).map(Event::Reply).into_iter().collect(),
                Some("mcp_tool_call") => item["tool"].as_str().map(|t| Event::Tool(t.to_string())).into_iter().collect(),
                _ => Vec::new(),
            }
        }
        Some("turn.completed") => usage(&v["usage"]).into_iter().chain([Event::Finished { error: false }]).collect(),
        Some("turn.failed") => vec![Event::Failed(text(&v["error"]["message"]).unwrap_or_else(|| "the turn failed".into()))],
        Some("error") => vec![Event::Failed(text(&v["message"]).unwrap_or_else(|| "an error".into()))],
        _ => Vec::new(),
    }
}

/// A run's outcome, from all its events.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Outcome {
    pub reply: Option<String>,
    pub tools: usize,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cost_usd: Option<f64>,
    /// `Some(error)` once the CLI said the run finished.
    pub finished: Option<bool>,
    pub failure: Option<String>,
    /// Whether Scrim's MCP server was reached, when the CLI says.
    pub scrim_connected: Option<bool>,
}

impl Outcome {
    pub fn add(&mut self, e: Event) {
        match e {
            Event::Started { scrim_connected } => self.scrim_connected = scrim_connected.or(self.scrim_connected),
            Event::Tool(_) => self.tools += 1,
            Event::Reply(r) => self.reply = Some(r),
            Event::Usage { input, output } => {
                self.input_tokens = input.or(self.input_tokens);
                self.output_tokens = output.or(self.output_tokens);
            }
            Event::Cost(c) => self.cost_usd = Some(c),
            Event::Finished { error } => self.finished = Some(error),
            Event::Failed(m) => self.failure = Some(m),
        }
    }
}

//! Profiles and reasoned ignores (`lint.md` §3–5): design intent stored with the document.

use serde_json::{Value, json};

use super::{Finding, Model, ancestors};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    /// Product UI and landing pages: every rule counts (the default).
    Ui,
    /// Posters, covers, editorial display type.
    Expressive,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Profile::Ui => "ui",
            Profile::Expressive => "expressive",
        }
    }
    pub fn parse(s: &str) -> Option<Profile> {
        match s {
            "ui" => Some(Profile::Ui),
            "expressive" => Some(Profile::Expressive),
            _ => None,
        }
    }
}

/// How a finding counts under a profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Active,
    Info,
    Off,
}

const ALWAYS_INFO: &[&str] = &["size-duplicated", "style-shorthand-conflict"];
const NEVER_RELAXED: &[&str] = &["low-contrast", "gray-on-color", "tiny-text", "root-overlap"];
const EXPRESSIVE: &[(&str, Mode)] =
    &[("oversized-h1", Mode::Off), ("side-tab", Mode::Off), ("clipped-overflow-container", Mode::Off), ("text-occluded", Mode::Info)];

pub fn mode(rule: &str, p: Profile) -> Mode {
    if ALWAYS_INFO.contains(&rule) {
        return Mode::Info;
    }
    if NEVER_RELAXED.contains(&rule) || p == Profile::Ui {
        return Mode::Active;
    }
    EXPRESSIVE.iter().find(|(r, _)| *r == rule).map_or(Mode::Active, |(_, m)| *m)
}

/// The profile string the tool descriptions carry.
pub fn describe_profiles() -> String {
    let changes: Vec<String> = EXPRESSIVE.iter().map(|(r, m)| format!("{r} {}", if *m == Mode::Off { "relaxed" } else { "informational" })).collect();
    format!(
        "'ui' = product UI and landing pages \u{2014} every rule counts (the default); 'expressive' = posters, covers, editorial display type \u{2014} relaxes rules aimed at UI type scales; contrast still counts ({})",
        changes.join(", ")
    )
}

/// A reasoned exception: a rule on a layer and everything in it, or (`node: None`) everywhere.
#[derive(Clone, Debug, PartialEq)]
pub struct Ignore<Id> {
    pub rule_id: String,
    pub node: Option<Id>,
    pub reason: String,
}

/// One edit of the ignores: `reason: None` removes; `index` places a new entry.
#[derive(Clone, Debug)]
pub struct IgnoreEdit<Id> {
    pub rule_id: String,
    pub node: Option<Id>,
    pub reason: Option<String>,
    pub index: Option<usize>,
}

/// The document's lint configuration: profiles per root (absent: `ui`) and the ordered ignores.
#[derive(Clone, Debug, PartialEq)]
pub struct LintConfig<Id> {
    pub profiles: Vec<(Id, Profile)>,
    pub ignores: Vec<Ignore<Id>>,
}

impl<Id> Default for LintConfig<Id> {
    fn default() -> Self {
        Self { profiles: Vec::new(), ignores: Vec::new() }
    }
}

impl<Id: Copy + PartialEq> LintConfig<Id> {
    pub fn profile(&self, root: Id) -> Profile {
        self.profiles.iter().find(|(r, _)| *r == root).map_or(Profile::Ui, |(_, p)| *p)
    }

    /// Set a root's profile; `ui` is stored as absent.
    pub fn set_profile(&mut self, root: Id, p: Profile) {
        self.profiles.retain(|(r, _)| *r != root);
        if p != Profile::Ui {
            self.profiles.push((root, p));
        }
    }

    /// `set_lint`'s ignore edit: re-reason in place, append (at `index`), or remove.
    pub fn apply_ignore(&mut self, e: IgnoreEdit<Id>) -> Result<(), String> {
        if e.rule_id.trim().is_empty() {
            return Err("a lint ignore needs a rule id".into());
        }
        let at = self.ignores.iter().position(|i| i.rule_id == e.rule_id && i.node == e.node);
        match (e.reason, at) {
            (Some(r), _) if r.trim().is_empty() => return Err(format!("ignoring {} needs a non-empty reason", e.rule_id)),
            (Some(r), Some(i)) => self.ignores[i].reason = r,
            (Some(r), None) => {
                let i = e.index.map_or(self.ignores.len(), |i| i.min(self.ignores.len()));
                self.ignores.insert(i, Ignore { rule_id: e.rule_id, node: e.node, reason: r });
            }
            (None, Some(i)) => {
                self.ignores.remove(i);
            }
            (None, None) => {}
        }
        Ok(())
    }

    /// The first ignore that covers a finding: the same rule, and no layer, the layer, one of its
    /// ancestors, or one of the layers the finding names.
    pub fn matching<M: Model<Id = Id>>(&self, m: &M, f: &Finding<Id>) -> Option<&Ignore<Id>> {
        let ups = ancestors(m, f.node);
        self.ignores.iter().find(|i| {
            i.rule_id == f.rule.id
                && match i.node {
                    None => true,
                    Some(n) => n == f.node || ups.contains(&n) || f.related.contains(&n),
                }
        })
    }

    pub fn ignores_json<M: Model<Id = Id>>(&self, m: &M) -> Vec<Value> {
        self.ignores
            .iter()
            .map(|i| {
                let mut v = json!({"ruleId": i.rule_id, "reason": i.reason});
                if let Some(n) = i.node {
                    v["nodeId"] = m.json_id(n);
                }
                v
            })
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty() && self.ignores.is_empty()
    }
}

//! One panel's turn as its log tells it: signals in, activity / draft / turn-end out.
//! Claude writes one line per content block (all carrying the final `stop_reason`), so an
//! end is held until the response's text has arrived or the CLI confirmed the end.

use super::events::ActivityKind;
use super::signal::{relative, Action, Signal};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Out {
    Activity {
        kind: ActivityKind,
        action: Option<Action>,
        target: Option<String>,
    },
    Draft(String),
    Ended {
        report: String,
        interrupted: bool,
    },
}

/// Turn state of one panel.
#[derive(Debug, Default)]
pub struct Fold {
    /// Whether the log marks turn ends; else drafts are sent for the idle heuristic.
    pub exact: bool,
    /// The turn's responses (id, text) in order.
    texts: Vec<(String, String)>,
    pending: Vec<String>,
    /// An end was seen: (response id, report given by the CLI).
    ending: Option<(Option<String>, Option<String>)>,
    last: Option<Out>,
}

impl Fold {
    pub fn new(exact: bool) -> Self {
        Self {
            exact,
            ..Self::default()
        }
    }

    pub fn apply(&mut self, signal: Signal, cwd: Option<&str>, out: &mut Vec<Out>) {
        match signal {
            Signal::Start => {
                self.end_now(out);
                self.texts.clear();
                self.pending.clear();
                self.activity(ActivityKind::Think, None, None, out);
            }
            Signal::Think => self.activity(ActivityKind::Think, None, None, out),
            Signal::Tool { id, action, target } => {
                if !self.pending.contains(&id) {
                    self.pending.push(id);
                }
                let target = target.map(|t| relative(t, cwd));
                self.activity(ActivityKind::Tool, Some(action), target, out);
            }
            Signal::ToolDone { id } => {
                self.pending.retain(|p| *p != id);
                if self.pending.is_empty() && self.ending.is_none() {
                    self.activity(ActivityKind::Think, None, None, out);
                }
            }
            Signal::Wait => self.activity(ActivityKind::Wait, None, None, out),
            Signal::Text { id, text } => {
                match self.texts.iter_mut().find(|(known, _)| *known == id) {
                    Some((_, said)) => said.push_str(&text),
                    None => self.texts.push((id, text)),
                }
                self.draft(out);
            }
            Signal::TextSet { id, text } => {
                match self.texts.iter_mut().find(|(known, _)| *known == id) {
                    Some((_, said)) => *said = text,
                    None => self.texts.push((id, text)),
                }
                self.draft(out);
            }
            Signal::End { id, report } => self.ending = Some((id, report)),
            Signal::EndConfirmed => self.end_now(out),
            Signal::Interrupted => {
                self.ending = None;
                self.texts.clear();
                self.pending.clear();
                out.push(Out::Ended {
                    report: String::new(),
                    interrupted: true,
                });
                self.activity(ActivityKind::Idle, None, None, out);
            }
        }
    }

    /// After a batch of lines: an end whose report is complete goes out now; one whose
    /// text is still missing waits for the next lines (or the confirmation).
    pub fn settle(&mut self, out: &mut Vec<Out>) {
        let ready = match &self.ending {
            Some((_, Some(_))) => true,
            Some((Some(id), None)) => self.text_of(id).is_some_and(|t| !t.trim().is_empty()),
            Some((None, None)) => true,
            None => false,
        };
        if ready {
            self.end_now(out);
        }
    }

    fn end_now(&mut self, out: &mut Vec<Out>) {
        let Some((id, report)) = self.ending.take() else {
            return;
        };
        let report = report
            .or_else(|| id.and_then(|id| self.text_of(&id).map(str::to_owned)))
            .or_else(|| self.texts.last().map(|(_, t)| t.clone()))
            .unwrap_or_default();
        self.texts.clear();
        self.pending.clear();
        out.push(Out::Ended {
            report: report.trim().to_owned(),
            interrupted: false,
        });
        self.activity(ActivityKind::Idle, None, None, out);
    }

    fn text_of(&self, id: &str) -> Option<&str> {
        self.texts
            .iter()
            .find(|(known, _)| known == id)
            .map(|(_, t)| t.as_str())
    }

    fn draft(&mut self, out: &mut Vec<Out>) {
        if self.exact {
            return;
        }
        if let Some((_, text)) = self.texts.last() {
            if !text.trim().is_empty() {
                out.push(Out::Draft(text.trim().to_owned()));
            }
        }
    }

    /// Activity changes only (the same tool twice in a row is sent once).
    fn activity(
        &mut self,
        kind: ActivityKind,
        action: Option<Action>,
        target: Option<String>,
        out: &mut Vec<Out>,
    ) {
        let next = Out::Activity {
            kind,
            action,
            target,
        };
        if self.last.as_ref() != Some(&next) {
            out.push(next.clone());
            self.last = Some(next);
        }
    }
}

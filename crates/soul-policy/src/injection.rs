//! External content is data. It is never instruction, and never authority.
//!
//! DECISIONS D25 states the rule and AC-25 is the red-line case: an injection
//! string arriving through an import line, a paste, or a file name must not
//! produce a tool plan and must not cause a connection to any URL it mentions.
//!
//! The enforcement is a type, not a filter. [`UntrustedText`] is the only way
//! prose from outside enters this crate, it does not implement `Display` and
//! cannot be `format!`ed into an instruction by accident, and the request
//! builder in `soul-egress` places it in a data slot that is separate from the
//! instruction slot. [`scan`] exists on top of that for the audit trail: it
//! reports what the content tried, so `injection.blocked` can be recorded, but
//! nothing downstream changes its behaviour based on what it says. A scanner
//! that were load-bearing would be a filter, and a filter can be evaded.

use std::fmt;

/// Where a piece of external content came in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExternalChannel {
    /// A line of `soul-import-v1` JSONL or a Telegram export.
    ImportLine,
    /// Text the user pasted into the draft box.
    Paste,
    /// A file name seen by the read-only directory scan.
    FileName,
}

impl ExternalChannel {
    pub const ALL: &'static [ExternalChannel] = &[
        ExternalChannel::ImportLine,
        ExternalChannel::Paste,
        ExternalChannel::FileName,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            ExternalChannel::ImportLine => "import_line",
            ExternalChannel::Paste => "paste",
            ExternalChannel::FileName => "file_name",
        }
    }
}

/// Prose that came from outside the user's own keyboard.
///
/// No `Display`, and `as_str` is named so that a reader of a call site can see
/// the escape hatch being taken. The point is not that the string is unusable
/// — the redactor has to read it — but that it cannot end up in an instruction
/// position without someone typing the words out.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UntrustedText {
    raw: String,
}

impl UntrustedText {
    pub fn new(raw: impl Into<String>) -> UntrustedText {
        UntrustedText { raw: raw.into() }
    }

    /// The bytes, for the redactor and for storage. Never for an instruction.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn char_count(&self) -> usize {
        self.raw.chars().count()
    }

    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }
}

/// What a piece of external content was trying to do. Recorded, not obeyed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InjectionSignal {
    /// "ignore previous instructions" and its relatives.
    InstructionOverride,
    /// A pretend system or developer turn.
    RoleImpersonation,
    /// A structure shaped like a tool call.
    ToolCallShape,
    /// A claim that the user already approved something.
    FabricatedApproval,
    /// A URL for the reader to fetch.
    EmbeddedUrl,
    /// A shell command.
    ShellCommand,
}

impl InjectionSignal {
    pub const fn as_str(self) -> &'static str {
        match self {
            InjectionSignal::InstructionOverride => "instruction_override",
            InjectionSignal::RoleImpersonation => "role_impersonation",
            InjectionSignal::ToolCallShape => "tool_call_shape",
            InjectionSignal::FabricatedApproval => "fabricated_approval",
            InjectionSignal::EmbeddedUrl => "embedded_url",
            InjectionSignal::ShellCommand => "shell_command",
        }
    }
}

impl fmt::Display for InjectionSignal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Substrings that mark an attempt to give instructions, in the two languages
/// v0.1 ships in.
const OVERRIDE_MARKERS: &[&str] = &[
    "ignore previous instructions",
    "ignore all prior instructions",
    "ignore the above",
    "disregard previous",
    "developer mode",
    "no restrictions",
    "忽略之前指令",
    "忽略以上",
    "忽略上面的",
    "从现在开始不需要",
];

const ROLE_MARKERS: &[&str] = &[
    "### system",
    "<!-- system",
    "[[assistant]]",
    "[[system]]",
    "\"role\":\"system\"",
    "\"role\": \"system\"",
    "你现在是系统管理员",
    "system prompt",
];

const TOOL_MARKERS: &[&str] = &["tool_call", "write_file", "function_call", "\"arguments\""];

const APPROVAL_MARKERS: &[&str] = &[
    "approve every",
    "without confirmation",
    "no confirmation needed",
    "用户已经同意",
    "不用再问",
    "不需要用户确认",
    "批准所有",
];

const SHELL_MARKERS: &[&str] = &["| sh", "|sh", "rm -rf", "curl ", "powershell", "cmd.exe"];

/// Report what `text` attempted. Used for the `injection.blocked` audit entry
/// and for nothing else.
pub fn scan(text: &UntrustedText) -> Vec<InjectionSignal> {
    let lowered = text.raw.to_lowercase();
    let mut signals = Vec::new();

    let mut mark = |markers: &[&str], signal: InjectionSignal| {
        if markers.iter().any(|marker| lowered.contains(marker)) {
            signals.push(signal);
        }
    };

    mark(OVERRIDE_MARKERS, InjectionSignal::InstructionOverride);
    mark(ROLE_MARKERS, InjectionSignal::RoleImpersonation);
    mark(TOOL_MARKERS, InjectionSignal::ToolCallShape);
    mark(APPROVAL_MARKERS, InjectionSignal::FabricatedApproval);
    mark(SHELL_MARKERS, InjectionSignal::ShellCommand);

    if !urls_in(text).is_empty() {
        signals.push(InjectionSignal::EmbeddedUrl);
    }

    signals.sort();
    signals.dedup();
    signals
}

pub fn looks_like_injection(text: &UntrustedText) -> bool {
    !scan(text).is_empty()
}

/// Every `http`/`https` URL the content mentions.
///
/// Extracted so a test can assert that none of them was ever contacted. The
/// guarantee itself does not come from this function: `NetGuard` refuses any
/// origin the user did not configure, so an unnoticed URL is refused too.
pub fn urls_in(text: &UntrustedText) -> Vec<String> {
    const TERMINATORS: &[char] = &[
        '"', '\'', '`', ' ', '\t', '<', '>', ')', ']', '}', ',', ';', '\\', '(', '|', '\n',
    ];
    let mut urls = Vec::new();
    let raw = &text.raw;
    let mut cursor = 0usize;

    while cursor < raw.len() {
        let Some(offset) = raw[cursor..].find("://") else {
            break;
        };
        let scheme_end = cursor + offset;
        let scheme_start = raw[cursor..scheme_end]
            .rfind(|c: char| !c.is_ascii_alphabetic())
            .map(|index| cursor + index + 1)
            .unwrap_or(cursor);
        let scheme = &raw[scheme_start..scheme_end];
        cursor = scheme_end + 3;
        if scheme != "http" && scheme != "https" {
            continue;
        }
        let rest = &raw[cursor..];
        let end = rest.find(TERMINATORS).unwrap_or(rest.len());
        let target = &rest[..end];
        cursor += end.max(1);
        if !target.is_empty() {
            urls.push(format!("{scheme}://{target}"));
        }
    }
    urls
}

//! Personalities, so two employees never sound the same. Each person starts with one that fits
//! their role, so a starting team already has its characters, until you pick another. It shapes
//! how they write, never how carefully they work.

use crate::store::{Company, Employee};
use anyhow::{Result, bail};

/// (key, label, how they write, a chat reply in that voice, so you can hear it before you pick)
pub const VOICES: &[(&str, &str, &str, &str)] = &[
    ("terse", "Terse", "You are terse. One line when one line will do. No greetings, no recaps.", "Done. Login works, tests pass."),
    ("warm", "Warm", "You are warm. You thank people, notice good work and keep the mood up, in a sentence or two.", "Login's in! Thanks for the clear ticket, that made it easy. Tests are green."),
    ("thorough", "Thorough", "You are thorough. You explain your reasoning and the trade-offs, in a short paragraph.", "Login is in. I went with magic links over passwords: less to store, and the email provider was already set up. The catch is a 15 minute expiry, which we can tune. Tests cover both paths."),
    ("dry", "Dry", "You have a dry, understated sense of humour. Short, plain, the odd wry aside.", "Login works. The tests agree, which is a first for this week."),
    ("blunt", "Blunt", "You are blunt. You say what's wrong and what to do about it, with no padding.", "Login works. The old session code was broken, so I replaced it. Review the auth middleware."),
    ("curious", "Curious", "You are curious. You often ask one sharp question before assuming.", "Login works with magic links. Quick question: should links expire after 15 minutes or an hour?"),
    ("upbeat", "Upbeat", "You are upbeat and like to get things moving. Energetic, short, practical.", "Login shipped! Tests green, on to signup next."),
    ("calm", "Calm", "You are calm and methodical. You lay things out step by step.", "Login is done. 1. Magic link sent by email. 2. Token checked on return. 3. Session set. Tests cover each step."),
];

/// The list for the app: key, label and a sample reply.
pub fn list() -> serde_json::Value {
    VOICES.iter().map(|(k, label, _, sample)| serde_json::json!({"key": k, "label": label, "sample": sample})).collect()
}

/// The personality a role starts with. Engineers vary between four, so a team of them still
/// sounds like several people.
pub fn for_role(role: &str, id: &str) -> &'static str {
    let r = role.to_lowercase();
    if ["cto", "lead", "head", "chief"].iter().any(|w| r.contains(w)) {
        "calm"
    } else if r.contains("review") || r.contains("qa") {
        "blunt"
    } else if r.contains("product") || r.contains("pm") {
        "curious"
    } else if r.contains("design") {
        "warm"
    } else if ["writ", "doc", "content", "growth", "market"].iter().any(|w| r.contains(w)) {
        "dry"
    } else {
        let h = id.bytes().fold(7u32, |h, b| h.wrapping_mul(31).wrapping_add(b as u32));
        ["terse", "upbeat", "thorough", "calm"][h as usize % 4]
    }
}

/// Their personality: the one you picked, or the one their role starts with.
pub fn of(c: &Company, e: &Employee) -> Result<String> {
    if let Some(k) = c.setting(&format!("voice.{}", e.id))?
        && VOICES.iter().any(|v| v.0 == k)
    {
        return Ok(k);
    }
    Ok(for_role(&e.role, &e.id).to_string())
}

pub fn set(c: &Company, id: &str, voice: &str) -> Result<()> {
    if !VOICES.iter().any(|v| v.0 == voice) {
        bail!("there's no {voice} personality");
    }
    c.set_setting(&format!("voice.{id}"), voice)
}

/// The bio plus how they write, for the top of every prompt.
pub fn intro(c: &Company, e: &Employee) -> Result<String> {
    let k = of(c, e)?;
    let how = VOICES.iter().find(|v| v.0 == k).map(|v| v.2).unwrap_or_default();
    Ok(format!("{} {how} Let it show in how you write chat, comments and summaries, never in the care you put into the work.", e.bio))
}


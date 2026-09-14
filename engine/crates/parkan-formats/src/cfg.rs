//! The `.cfg` text format: `object NAME … end` blocks of `key = value` lines, and the
//! two lists a mission's progression reads from it, its objectives and its messages.
//!
//! Ports `openparkan.mission.load_cfg`, `mission.objectives` and `briefing.messages`.
//! See `docs/04-missions.md`, `docs/21-briefing.md` and `docs/34-progression.md`.

use crate::cursor::latin1;

/// The two `mission.cfg` objects the objective list is built from, primary first
/// (`iron3d.dll:0x1006a780`).
pub const PRIMARY_OBJECTIVES: &str = "primary_objectives";
pub const BONUS_OBJECTIVES: &str = "bonus_objectives";

/// One `object NAME … end` block: its properties in the order they first appear.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Block {
    pub name: String,
    pub properties: Vec<(String, String)>,
}

impl Block {
    /// A property by its exact key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.properties.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn set(&mut self, key: String, value: String) {
        match self.properties.iter_mut().find(|(k, _)| *k == key) {
            Some(slot) => slot.1 = value,
            None => self.properties.push((key, value)),
        }
    }
}

/// Every block of a file, in the order each name first appears.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Blocks(pub Vec<Block>);

impl Blocks {
    /// A block by its exact name.
    pub fn get(&self, name: &str) -> Option<&Block> {
        self.0.iter().find(|b| b.name == name)
    }
}

/// Whitespace as the reference reader's `str.strip` and `str.split` see it, over the
/// Latin-1 range the file is decoded into.
fn is_space(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\u{0b}' | '\u{0c}' | '\r' | '\u{1c}'..='\u{1f}' | ' ' | '\u{85}' | '\u{a0}')
}

/// The line breaks the reference reader's `str.splitlines` splits on, over Latin-1.
fn is_break(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{0b}' | '\u{0c}' | '\u{1c}' | '\u{1d}' | '\u{1e}' | '\u{85}')
}

fn lines(text: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut chars = text.char_indices().peekable();
    while let Some((i, c)) = chars.next() {
        if is_break(c) {
            out.push(&text[start..i]);
            let mut end = i + c.len_utf8();
            if c == '\r'
                && let Some(&(j, '\n')) = chars.peek()
            {
                chars.next();
                end = j + 1;
            }
            start = end;
        }
    }
    if start < text.len() {
        out.push(&text[start..]);
    }
    out
}

/// Parse a `.cfg` file's bytes, decoded as Latin-1.
///
/// Text from a `#` on is a comment. A header is the word `object` on its own:
/// `objective1 = …` starts with the same six letters and is a property. A repeated
/// block name adds to the first block of that name, and a repeated key keeps its
/// place and takes the later value. Values lose surrounding quotes.
pub fn parse(data: &[u8]) -> Blocks {
    let text = latin1(data);
    let mut blocks: Vec<Block> = Vec::new();
    let mut current: Option<usize> = None;
    for raw in lines(&text) {
        let line = raw.split('#').next().unwrap_or("").trim_matches(is_space);
        if line.is_empty() {
            continue;
        }
        let (first, rest) = match line.find(is_space) {
            Some(at) => (&line[..at], Some(line[at..].trim_matches(is_space))),
            None => (line, None),
        };
        if line.eq_ignore_ascii_case("end") {
            current = None;
        } else if first.eq_ignore_ascii_case("object") {
            let name = rest.unwrap_or("").to_owned();
            current = Some(match blocks.iter().position(|b| b.name == name) {
                Some(at) => at,
                None => {
                    blocks.push(Block { name, properties: Vec::new() });
                    blocks.len() - 1
                }
            });
        } else if let Some(at) = current
            && let Some((key, value)) = line.split_once('=')
        {
            let value = value.trim_matches(is_space).trim_matches('"');
            blocks[at].set(key.trim_matches(is_space).to_owned(), value.to_owned());
        }
    }
    Blocks(blocks)
}

/// One line of a mission's objective list. A script names it by its place: the
/// primary objectives in file order, then the bonus ones, which are exempt from the
/// completion test (`iron3d.dll:0x1006b130`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Objective {
    pub text: String,
    pub exempt: bool,
}

/// The objective list a `mission.cfg` gives, in script order.
pub fn objectives(blocks: &Blocks) -> Vec<Objective> {
    let of = |name: &str, exempt: bool| {
        blocks
            .get(name)
            .into_iter()
            .flat_map(|b| b.properties.iter())
            .map(move |(_, text)| Objective { text: text.clone(), exempt })
    };
    of(PRIMARY_OBJECTIVES, false).chain(of(BONUS_OBJECTIVES, true)).collect()
}

/// One line of in-mission dialogue from `messages.cfg`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub name: String,
    /// `message_index`: the id a script asks for, not a position.
    pub index: i64,
    pub text_id: String,
    pub voice_id: String,
    /// `info_system`: the history files it as kind 4 rather than 3
    /// (`iron3d.dll:0x10095519`); what that changes is not read.
    pub info_system: bool,
}

/// A number as the reference reader takes it: a float, 0 when it does not parse.
fn number(text: &str) -> f64 {
    text.trim_matches(is_space).parse().unwrap_or(0.0)
}

fn flag(text: &str) -> bool {
    text.trim_matches(is_space).eq_ignore_ascii_case("true")
}

/// Every block of a `messages.cfg`, in file order.
pub fn messages(blocks: &Blocks) -> Vec<Message> {
    blocks
        .0
        .iter()
        .map(|b| {
            let get = |k: &str| b.get(k).unwrap_or("");
            Message {
                name: b.name.clone(),
                index: number(b.get("message_index").unwrap_or("-1")).trunc() as i64,
                text_id: get("text_resource").to_owned(),
                voice_id: get("voice_resource").to_owned(),
                info_system: flag(get("info_system")),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CFG: &str = "# the objectives, and a message beside them\r\n\
        object primary_objectives\r\n\
        \x20 objective1  = \"1. Destroy all the targets\"   # a comment\r\n\
        \x20 objective2  = \"2. Capture the neutral warbots\"\r\n\
        end\r\n\
        \r\n\
        object bonus_objectives\r\n\
        \x20 bonus = \"Stay alive\"\r\n\
        end\r\n\
        OBJECT\tmission\r\n\
        \x20  only_briefing = false\r\n\
        \x20  only_briefing = true\r\n\
        End\r\n\
        stray = ignored\r\n\
        object primary_objectives\r\n\
        \x20 objective3 = \"3. Destroy the enemy\"\r\n\
        end\r\n";

    #[test]
    fn blocks_keep_first_appearance_order_and_an_objective_line_is_a_property() {
        let b = parse(CFG.as_bytes());
        let names: Vec<&str> = b.0.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names, ["primary_objectives", "bonus_objectives", "mission"]);
        let primary = b.get("primary_objectives").unwrap();
        assert_eq!(primary.get("objective1"), Some("1. Destroy all the targets"));
        assert_eq!(primary.properties.len(), 3, "a repeated block adds to the first");
        assert_eq!(b.get("mission").unwrap().properties, vec![("only_briefing".into(), "true".into())]);
    }

    #[test]
    fn objectives_are_primary_then_bonus() {
        let list = objectives(&parse(CFG.as_bytes()));
        let texts: Vec<(&str, bool)> = list.iter().map(|o| (o.text.as_str(), o.exempt)).collect();
        assert_eq!(
            texts,
            [
                ("1. Destroy all the targets", false),
                ("2. Capture the neutral warbots", false),
                ("3. Destroy the enemy", false),
                ("Stay alive", true)
            ]
        );
    }

    #[test]
    fn messages_read_their_index_resources_and_flag() {
        let text = "object message1\n message_index = 11\n text_resource = \"T01_I01\"\n \
                    voice_resource = \"T01_I01\"\nend\nobject message2\n message_index = 2.9\n \
                    info_system = TRUE \nend\nobject message3\n message_index = x\nend\n\
                    object message4\nend\n";
        let m = messages(&parse(text.as_bytes()));
        assert_eq!(m.len(), 4);
        assert_eq!((m[0].index, m[0].text_id.as_str(), m[0].voice_id.as_str()), (11, "T01_I01", "T01_I01"));
        assert!(!m[0].info_system && m[1].info_system);
        assert_eq!([m[1].index, m[2].index, m[3].index], [2, 0, -1]);
    }

    #[test]
    fn lines_break_as_the_reference_reader_splits_them() {
        assert_eq!(lines("a\r\nb\rc\nd\u{85}e"), ["a", "b", "c", "d", "e"]);
        assert_eq!(lines("a\n\nb\n"), ["a", "", "b"]);
    }
}

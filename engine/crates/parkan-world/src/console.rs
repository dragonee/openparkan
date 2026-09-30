//! The game's debug console as a mission's `script` block reaches it: function 57 hands the
//! line its script names to the console, which cuts it into a command and its fields and runs
//! it. Three of the console's ten commands ship in the install's 20 lines: `create`, `bcreate`
//! and `death`. See `docs/15-behaviour.md`, "Channel 2 runs a line of the mission's `script`
//! block" and "What the console's `create`, `bcreate` and `death` do".

/// Where `create` and `bcreate` read their file from: `units/auto/` and the file's name
/// (`iron3d.dll:0x10103ef4`, `%s%s`), as the install spells the directory.
pub const UNITS_AUTO: &str = "UNITS\\AUTO\\";

/// How far over the ground `create` stands its unit: the level's probe at its x, y plus 2
/// (`0x10077562`, the float at `0x100e5c0c`).
pub const CREATE_LIFT: f32 = 2.0;

/// A console line, read as the console reads it. Every number is a whole one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Command {
    /// `create(x, y, heading, clan, file, …)` (`0x1003d280`): a unit of `UNITS\AUTO\<file>`
    /// for the clan the mission's clan table numbers so, standing [`CREATE_LIFT`] over the
    /// ground at (x, y), turned `heading` radians about z. The help text calls the third field
    /// `<z>`; the handler turns the unit by it.
    Create { x: i32, y: i32, heading: i32, clan: i32, file: String },
    /// `bcreate(x, y, z, clan, file, …)` (`0x1003d6a0`): a building of `UNITS\AUTO\<file>`
    /// for that clan, its matrix unturned with (x, y, z) its translation.
    BCreate { x: i32, y: i32, z: i32, clan: i32, file: String },
    /// `death(x, y, r, delay)` (`0x1003db10`): the scenery whose sphere meets the sphere of
    /// radius `r` about the ground at (x, y) killed, at once. The delay is read and never used.
    Death { x: i32, y: i32, radius: i32 },
}

/// A field as the C library's `atol` reads it (`0x100b4730`): white space skipped, a sign,
/// then digits up to the first character that is not one; 0 when there are none.
fn whole(field: &str) -> i32 {
    let s = field.trim_start_matches([' ', '\t', '\n', '\r', '\x0b', '\x0c']);
    let (negative, s) = match s.as_bytes().first() {
        Some(b'-') => (true, &s[1..]),
        Some(b'+') => (false, &s[1..]),
        _ => (false, s),
    };
    let magnitude = s
        .bytes()
        .take_while(u8::is_ascii_digit)
        .fold(0i32, |n, d| n.wrapping_mul(10).wrapping_add(i32::from(d - b'0')));
    if negative { magnitude.wrapping_neg() } else { magnitude }
}

/// The pieces of `s` between any of `delimiters`, empty ones passed over, as `strtok` hands
/// them out.
fn pieces<'a>(s: &'a str, delimiters: &'a [char]) -> impl Iterator<Item = &'a str> {
    s.split(delimiters).filter(|p| !p.is_empty())
}

/// A console line as a command (`0x1003ca50`): the line is cut at every `(` and `)`, the first
/// piece names the command, matched without regard to case (`_stricmp`), and the second is
/// its fields. Each handler cuts those at every comma, space and `)` (`0x10103f00`) and does
/// nothing unless all its fields are there: six for `create` and `bcreate`, whose sixth is
/// read and dropped, and four for `death`. `None` for another of the console's commands, or a
/// line its handler would pass over.
pub fn parse(line: &str) -> Option<Command> {
    let mut outer = pieces(line, &['(', ')']);
    let name = outer.next()?;
    let fields: Vec<&str> = pieces(outer.next()?, &[',', ' ', ')']).collect();
    if name.eq_ignore_ascii_case("create") || name.eq_ignore_ascii_case("bcreate") {
        let [x, y, z, clan, file, _, ..] = fields.as_slice() else { return None };
        let (x, y, z, clan, file) = (whole(x), whole(y), whole(z), whole(clan), (*file).to_owned());
        Some(if name.eq_ignore_ascii_case("create") {
            Command::Create { x, y, heading: z, clan, file }
        } else {
            Command::BCreate { x, y, z, clan, file }
        })
    } else if name.eq_ignore_ascii_case("death") {
        let [x, y, radius, _, ..] = fields.as_slice() else { return None };
        Some(Command::Death { x: whole(x), y: whole(y), radius: whole(radius) })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_lines_read_as_whole_numbers_and_the_trailing_fields_are_dropped() {
        assert_eq!(
            parse("create(918, 683, 10, 4, 22lwhl1.dat, 0)"),
            Some(Command::Create { x: 918, y: 683, heading: 10, clan: 4, file: "22lwhl1.dat".into() })
        );
        assert_eq!(
            parse("bcreate(1246, 1051, 10, 0, teleport.dat, 0)"),
            Some(Command::BCreate { x: 1246, y: 1051, z: 10, clan: 0, file: "teleport.dat".into() })
        );
        assert_eq!(
            parse("death(1246, 1051, 100, 0 )"),
            Some(Command::Death { x: 1246, y: 1051, radius: 100 })
        );
    }

    #[test]
    fn a_handler_passes_over_a_line_short_of_a_field_and_the_console_over_its_other_commands() {
        assert_eq!(parse("create(918, 683, 10, 4, 22lwhl1.dat)"), None, "the sixth is required");
        assert_eq!(parse("death(1246, 1051, 100)"), None, "so is the delay");
        assert_eq!(parse("kill(self)"), None);
        assert_eq!(parse("create"), None, "no fields at all");
    }

    #[test]
    fn a_name_matches_without_regard_to_case_and_a_field_reads_as_atol_does() {
        assert_eq!(
            parse("CREATE( -5,+7 ,1.9, 2x, a.dat,0)"),
            Some(Command::Create { x: -5, y: 7, heading: 1, clan: 2, file: "a.dat".into() })
        );
        assert_eq!(whole("abc"), 0);
        assert_eq!(whole("  42.5"), 42);
    }
}

//! `.wea`, a wear: the material palette a mesh or terrain layer indexes, and
//! any lightmaps. Plain text: a count, then `index name` pairs, then keyword
//! tables.

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Wear {
    pub materials: Vec<String>,
    pub lightmaps: Vec<String>,
}

fn is_int(t: &str) -> bool {
    let digits = t.strip_prefix('-').unwrap_or(t);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// A count then `index name` pairs from `start`; the table and where reading stopped.
fn table(tokens: &[&str], start: usize) -> (Vec<String>, usize) {
    let Some(count) = tokens.get(start).filter(|t| is_int(t)).and_then(|t| t.parse::<i64>().ok()) else {
        return (Vec::new(), start);
    };
    let count = usize::try_from(count).unwrap_or(0);
    let mut names = vec![String::new(); count];
    let mut i = start + 1;
    while i + 1 < tokens.len() && is_int(tokens[i]) {
        if let Ok(index) = usize::try_from(tokens[i].parse::<i64>().unwrap_or(-1))
            && index < count
        {
            names[index] = tokens[i + 1].to_owned();
        }
        i += 2;
    }
    (names, i)
}

pub fn parse(blob: &[u8]) -> Wear {
    let text: String = blob.iter().map(|&b| char::from(b)).collect();
    let tokens: Vec<&str> = text.split_whitespace().collect();
    let (materials, mut at) = table(&tokens, 0);
    let mut lightmaps = Vec::new();
    while at < tokens.len() {
        let keyword = tokens[at].to_ascii_uppercase();
        let (found, next) = table(&tokens, at + 1);
        if keyword == "LIGHTMAPS" {
            lightmaps = found.clone();
        }
        at = if found.is_empty() { next + 1 } else { next };
    }
    Wear { materials, lightmaps }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wear_lists_materials_by_index_then_lightmaps() {
        let w = parse(b"3\n0 B_S0\n2 L00\n1 L02\nLIGHTMAPS\n1\n0 LM1\n");
        assert_eq!(w.materials, vec!["B_S0", "L02", "L00"]);
        assert_eq!(w.lightmaps, vec!["LM1"]);
    }
}

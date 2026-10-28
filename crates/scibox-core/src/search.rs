//! Search: find a category, a unit, a formula, or a tab by its name.

use crate::formulas::DOMAINS;
use crate::units::GROUPS;

pub enum Target {
    /// A tab index.
    Tab(usize),
    /// A converter tab, a category, and an optional "from" unit.
    Category(usize, usize, Option<usize>),
    /// A domain and a formula.
    Formula(usize, usize),
}

pub struct Entry {
    pub label: String,
    pub detail: String,
    pub target: Target,
    /// The lowercase search text: the label, then the detail.
    key: String,
    /// The length of the label part of `key`, in bytes.
    label_len: usize,
}

impl Entry {
    fn new(label: String, detail: String, target: Target) -> Self {
        let label_len = label.to_lowercase().len();
        let key = format!("{}  {}", label, detail).to_lowercase();
        Entry {
            label,
            detail,
            target,
            key,
            label_len,
        }
    }
}

/// Make the list of all the entries. `tabs` is the list of the tabs that are not converter tabs.
pub fn entries(tabs: &[(usize, &str)]) -> Vec<Entry> {
    let mut out = Vec::new();
    // Formulas first: if two entries have the same score, the formula comes first.
    for (d, dom) in DOMAINS.iter().enumerate() {
        for (i, fm) in dom.formulas.iter().enumerate() {
            let detail = format!("Formula · {} · {}", dom.name, fm.equation);
            out.push(Entry::new(fm.name.into(), detail, Target::Formula(d, i)));
        }
    }
    for (t, g) in GROUPS.iter().enumerate() {
        for (c, cat) in g.categories.iter().enumerate() {
            let detail = format!("Convert · {}", g.name);
            out.push(Entry::new(
                cat.name.into(),
                detail,
                Target::Category(t, c, None),
            ));
            for (u, unit) in cat.units.iter().enumerate() {
                let label = format!("{} ({})", unit.name, unit.symbol);
                let detail = format!("Unit · {}", cat.name);
                out.push(Entry::new(label, detail, Target::Category(t, c, Some(u))));
            }
        }
    }
    for &(t, name) in tabs {
        out.push(Entry::new(name.into(), "Tab".into(), Target::Tab(t)));
    }
    out
}

/// The indices of the entries that agree with the query, best first.
pub fn find(entries: &[Entry], query: &str) -> Vec<usize> {
    let mut scored: Vec<(usize, usize)> = entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| score(e, query).map(|s| (s, i)))
        .collect();
    // The sort is stable. Thus, entries with the same score keep their order.
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    scored.into_iter().map(|(_, i)| i).collect()
}

/// The score of an entry for the query, or `None` if a word of the query is not in the entry.
/// A word at the start of the label gets the highest score. A scattered match gets a low score.
fn score(e: &Entry, query: &str) -> Option<usize> {
    let mut total = 0;
    for word in query.to_lowercase().split_whitespace() {
        total += match e.key.find(word) {
            Some(0) => 400,
            Some(i) if i < e.label_len && e.key[..i].ends_with([' ', '(']) => 300,
            Some(i) if i < e.label_len => 200,
            Some(_) => 100,
            None if is_subsequence(word, &e.key[..e.label_len]) => 20,
            None => return None,
        };
    }
    Some(total)
}

fn is_subsequence(word: &str, text: &str) -> bool {
    let mut chars = text.chars();
    word.chars().all(|w| chars.any(|c| c == w))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search() {
        let all = entries(&[(6, "Calc")]);
        assert_eq!(find(&all, "").len(), all.len());
        for (query, expected) in [
            ("dbm", "Decibel-milliwatt (dBm)"),
            ("ohm law", "Ohm's law"),
            ("fspl", "Free-space path loss"),
            ("calc", "Calc"),
        ] {
            let found: Vec<&str> = find(&all, query)
                .iter()
                .take(5)
                .map(|&i| all[i].label.as_str())
                .collect();
            assert!(found.contains(&expected), "{query}: {found:?}");
        }
    }
}

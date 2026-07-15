// Responsibility: in-memory-session-snapshot-line-delta
use super::LineDelta;

pub(super) fn text_line_delta(base: Option<&str>, current: Option<&str>) -> LineDelta {
    let old = base
        .map(|text| text.lines().collect::<Vec<_>>())
        .unwrap_or_default();
    let new = current
        .map(|text| text.lines().collect::<Vec<_>>())
        .unwrap_or_default();
    if old.is_empty() {
        return LineDelta {
            added: numbered(&new, 0),
            removed: Vec::new(),
        };
    }
    if new.is_empty() {
        return LineDelta {
            added: Vec::new(),
            removed: numbered(&old, 0),
        };
    }
    if old.len().saturating_mul(new.len()) > 1_000_000 {
        return bounded_middle_delta(&old, &new);
    }
    lcs_line_delta(&old, &new)
}

fn lcs_line_delta(old: &[&str], new: &[&str]) -> LineDelta {
    let columns = new.len() + 1;
    let mut lengths = vec![0usize; (old.len() + 1) * columns];
    for old_index in (0..old.len()).rev() {
        for new_index in (0..new.len()).rev() {
            let index = old_index * columns + new_index;
            lengths[index] = if old[old_index] == new[new_index] {
                lengths[(old_index + 1) * columns + new_index + 1] + 1
            } else {
                lengths[(old_index + 1) * columns + new_index]
                    .max(lengths[old_index * columns + new_index + 1])
            };
        }
    }
    let mut delta = LineDelta::default();
    let (mut old_index, mut new_index) = (0usize, 0usize);
    while old_index < old.len() || new_index < new.len() {
        if old_index < old.len() && new_index < new.len() && old[old_index] == new[new_index] {
            old_index += 1;
            new_index += 1;
        } else if new_index < new.len()
            && (old_index == old.len()
                || lengths[old_index * columns + new_index + 1]
                    >= lengths[(old_index + 1) * columns + new_index])
        {
            delta
                .added
                .push((new_index + 1, new[new_index].to_string()));
            new_index += 1;
        } else {
            delta
                .removed
                .push((old_index + 1, old[old_index].to_string()));
            old_index += 1;
        }
    }
    delta
}

fn bounded_middle_delta(old: &[&str], new: &[&str]) -> LineDelta {
    let prefix = old
        .iter()
        .zip(new.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    LineDelta {
        added: numbered(&new[prefix..new.len() - suffix], prefix),
        removed: numbered(&old[prefix..old.len() - suffix], prefix),
    }
}

fn numbered(lines: &[&str], offset: usize) -> Vec<(usize, String)> {
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| (offset + index + 1, (*line).to_string()))
        .collect()
}

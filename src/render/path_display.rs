// Responsibility: long-symbol-path-readable-display
use crate::model::StructuralEdge;
use sha2::{Digest, Sha256};

const LONG_SYMBOL_ANCHOR_THRESHOLD: usize = 80;
const MAX_REPEATED_PATH_CHARS: usize = 88;

/// Readable-only aliases for a long exact symbol anchor. The report model and
/// JSON keep canonical repository-relative paths; this context only removes
/// repetition after the full anchor has been printed in the readable header.
pub(crate) struct AnchorPathDisplay<'a> {
    anchor: &'a str,
    file: &'a str,
    directory: Option<&'a str>,
    compact: bool,
}

impl<'a> AnchorPathDisplay<'a> {
    pub(crate) fn new(anchor: &'a str) -> Self {
        let file = anchor.split_once('#').map_or(anchor, |(file, _)| file);
        let directory = file.rsplit_once('/').map(|(directory, _)| directory);
        let compact = anchor.contains('#')
            && directory.is_some()
            && anchor.chars().count() > LONG_SYMBOL_ANCHOR_THRESHOLD;
        Self {
            anchor,
            file,
            directory,
            compact,
        }
    }

    pub(crate) fn compact(&self) -> bool {
        self.compact
    }

    pub(crate) fn header_suffix(&self) -> &'static str {
        if self.compact {
            " (aliases: @anchor, @file, ./ sibling, @from/@to)"
        } else {
            ""
        }
    }

    pub(crate) fn path(&self, path: &str) -> String {
        if !self.compact {
            return path.to_string();
        }
        if path == self.anchor {
            return "@anchor".to_string();
        }
        let (file, symbol) = split_symbol(path);
        if file == self.file {
            return match symbol {
                Some(symbol) => format!("@file#{symbol}"),
                None => "@file".to_string(),
            };
        }
        let Some(directory) = self.directory else {
            return bounded_path_label(path);
        };
        let prefix = format!("{directory}/");
        let Some(relative) = file.strip_prefix(&prefix) else {
            return bounded_path_label(path);
        };
        bounded_path_label(&match symbol {
            Some(symbol) => format!("./{relative}#{symbol}"),
            None => format!("./{relative}"),
        })
    }

    pub(crate) fn edge_location_path(&self, edge: &StructuralEdge, path: &str) -> String {
        if !self.compact {
            return path.to_string();
        }
        if endpoint_file(&edge.from) == path {
            return "@from".to_string();
        }
        if endpoint_file(&edge.to) == path {
            return "@to".to_string();
        }
        self.path(path)
    }
}

fn bounded_path_label(path: &str) -> String {
    if path.chars().count() <= MAX_REPEATED_PATH_CHARS {
        return path.to_string();
    }
    let tail = path
        .chars()
        .rev()
        .take(68)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    let digest = format!("{:x}", Sha256::digest(path.as_bytes()));
    format!("…{tail}~{}", &digest[..10])
}

fn split_symbol(path: &str) -> (&str, Option<&str>) {
    path.split_once('#')
        .map_or((path, None), |(file, symbol)| (file, Some(symbol)))
}

fn endpoint_file(endpoint: &str) -> &str {
    split_symbol(endpoint).0
}

#[cfg(test)]
mod tests {
    use super::bounded_path_label;

    #[test]
    fn long_external_labels_are_bounded_and_collision_resistant() {
        let suffix = "/src/features/financial-reporting/shared/consumer.ts#consume";
        let first = bounded_path_label(&format!("packages/first-very-long-owner{suffix}"));
        let second = bounded_path_label(&format!("packages/second-very-long-owner{suffix}"));
        assert!(first.chars().count() <= 88, "{first}");
        assert!(second.chars().count() <= 88, "{second}");
        assert_ne!(first, second);
    }
}

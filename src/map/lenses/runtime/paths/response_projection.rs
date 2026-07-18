// Responsibility: response-output call roles and primary-parameter field provenance
use crate::map::{code_shape_without_literal_content, identifier_ranges, runtime_code_lines};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn response_output_call(body: &str, name: &str) -> bool {
    let code = code_text(body);
    let returns = return_expressions(&code);
    returns
        .iter()
        .any(|expression| call_has_only_output_parents(expression, name))
        || assigned_call_results(&code, name).iter().any(|local| {
            returns
                .iter()
                .any(|expression| value_has_only_output_parents(expression, local))
        })
}

pub(crate) fn explicitly_omitted_fields(body: &str) -> Vec<String> {
    let code = code_text(body);
    let Some(primary) = primary_parameter(&code) else {
        return Vec::new();
    };
    let returns = return_expressions(&code);
    if !returns.iter().any(|expression| {
        expression
            .trim_start_matches([' ', '\n', '\t', '('])
            .starts_with('{')
    }) {
        return Vec::new();
    }
    let (receivers, derived) = parameter_provenance(&code, &primary);
    let referenced = fields_read_from(&code, &receivers);
    let mut returned = BTreeSet::new();
    for expression in returns {
        returned.extend(fields_read_from(expression, &receivers));
        for (local, fields) in &derived {
            if value_identifier_occurs(expression, local) {
                returned.extend(fields.iter().cloned());
            }
        }
    }
    referenced.difference(&returned).cloned().collect()
}

fn code_text(body: &str) -> String {
    runtime_code_lines(body)
        .into_iter()
        .map(|(_, line)| code_shape_without_literal_content(&line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn primary_parameter(code: &str) -> Option<String> {
    let open = code.find('(')?;
    let close = crate::repo::js_balanced_pattern_end(code, open)?;
    let first = crate::repo::js_split_top_level_commas(&code[open + 1..close])
        .into_iter()
        .next()?;
    first
        .trim_start()
        .chars()
        .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$'))
        .collect::<String>()
        .pipe_nonempty()
}

trait NonemptyString {
    fn pipe_nonempty(self) -> Option<String>;
}

impl NonemptyString for String {
    fn pipe_nonempty(self) -> Option<String> {
        (!self.is_empty()).then_some(self)
    }
}

fn parameter_provenance(
    code: &str,
    primary: &str,
) -> (BTreeSet<String>, BTreeMap<String, BTreeSet<String>>) {
    let mut receivers = BTreeSet::from([primary.to_string()]);
    let mut derived = BTreeMap::new();
    let statements = code.split([';', '\n']).collect::<Vec<_>>();
    for _ in 0..statements.len().max(1) {
        let mut changed = false;
        for statement in &statements {
            let Some((local, rhs)) = local_assignment(statement) else {
                continue;
            };
            if receivers.contains(rhs.trim()) {
                changed |= receivers.insert(local.to_string());
                continue;
            }
            let fields = fields_read_from(rhs, &receivers);
            if !fields.is_empty() && derived.get(local) != Some(&fields) {
                derived.insert(local.to_string(), fields);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    (receivers, derived)
}

fn local_assignment(statement: &str) -> Option<(&str, &str)> {
    let statement = statement.trim();
    let tail = ["const ", "let ", "var "]
        .iter()
        .find_map(|prefix| statement.strip_prefix(prefix))?;
    let name_end = tail
        .find(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$')))
        .unwrap_or(tail.len());
    let local = &tail[..name_end];
    let equals = tail[name_end..].find('=')? + name_end;
    Some((local, &tail[equals + 1..]))
}

fn fields_read_from(code: &str, receivers: &BTreeSet<String>) -> BTreeSet<String> {
    let mut fields = BTreeSet::new();
    for receiver in receivers {
        for (_, end) in identifier_ranges(code, receiver) {
            let mut tail = code[end..].trim_start();
            tail = tail
                .strip_prefix("?.")
                .or_else(|| tail.strip_prefix('.'))
                .unwrap_or("");
            let field = tail
                .chars()
                .take_while(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$'))
                .collect::<String>();
            if !field.is_empty() {
                fields.insert(field);
            }
        }
    }
    fields
}

fn value_identifier_occurs(code: &str, name: &str) -> bool {
    identifier_ranges(code, name).any(|(start, end)| {
        let before = code[..start].trim_end();
        let after = code[end..].trim_start();
        !before.ends_with('.') && !after.starts_with(':')
    })
}

fn return_expressions(code: &str) -> Vec<&str> {
    let mut out = Vec::new();
    for (_, end) in identifier_ranges(code, "return") {
        let start = end
            + code[end..]
                .len()
                .saturating_sub(code[end..].trim_start().len());
        if start >= code.len() || code[start..].starts_with([';', '\n', '}']) {
            continue;
        }
        let finish = expression_end(code, start);
        if finish > start {
            out.push(code[start..finish].trim());
        }
    }
    out
}

fn expression_end(code: &str, start: usize) -> usize {
    let mut stack = Vec::new();
    for (relative, ch) in code[start..].char_indices() {
        match ch {
            '(' => stack.push(')'),
            '[' => stack.push(']'),
            '{' => stack.push('}'),
            ')' | ']' | '}' if stack.last() == Some(&ch) => {
                stack.pop();
            }
            ';' | '\n' if stack.is_empty() => return start + relative,
            '}' if stack.is_empty() => return start + relative,
            _ => {}
        }
    }
    code.len()
}

fn call_has_only_output_parents(expression: &str, name: &str) -> bool {
    for (start, end) in identifier_ranges(expression, name) {
        let after = end
            + expression[end..]
                .len()
                .saturating_sub(expression[end..].trim_start().len());
        if expression.as_bytes().get(after) != Some(&b'(') {
            continue;
        }
        let parents = parent_calls_before(expression, start);
        if parents.iter().all(|parent| output_wrapper_name(parent)) {
            return true;
        }
    }
    false
}

fn assigned_call_results<'a>(code: &'a str, name: &str) -> Vec<&'a str> {
    code.split([';', '\n'])
        .filter_map(local_assignment)
        .filter_map(|(local, rhs)| {
            identifier_ranges(rhs, name)
                .any(|(_, end)| rhs[end..].trim_start().starts_with('('))
                .then_some(local)
        })
        .collect()
}

fn value_has_only_output_parents(expression: &str, name: &str) -> bool {
    identifier_ranges(expression, name).any(|(start, end)| {
        let before = expression[..start].trim_end();
        let after = expression[end..].trim_start();
        !before.ends_with('.')
            && !after.starts_with(':')
            && parent_calls_before(expression, start)
                .iter()
                .all(|parent| output_wrapper_name(parent))
    })
}

fn parent_calls_before(expression: &str, target: usize) -> Vec<String> {
    let mut stack: Vec<Option<String>> = Vec::new();
    for (index, ch) in expression[..target].char_indices() {
        match ch {
            '(' => stack.push(callee_before(expression, index)),
            ')' => {
                stack.pop();
            }
            _ => {}
        }
    }
    stack.into_iter().flatten().collect()
}

fn callee_before(expression: &str, open: usize) -> Option<String> {
    let prefix = expression[..open].trim_end();
    let start = prefix
        .rfind(|ch: char| !(ch.is_ascii_alphanumeric() || matches!(ch, '_' | '$' | '.')))
        .map_or(0, |index| index + 1);
    let name = prefix[start..].trim_matches('.');
    (!name.is_empty()).then(|| name.to_string())
}

fn output_wrapper_name(name: &str) -> bool {
    matches!(
        name,
        "NextResponse.json" | "Response.json" | "res.json" | "reply.send" | "ctx.json" | "c.json"
    ) || super::classify::transformation_name(name.rsplit('.').next().unwrap_or(name))
}

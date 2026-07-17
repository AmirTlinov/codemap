// Responsibility: github-actions-job-step-structure
use crate::map::{CiRunStep, ci_run_steps_with_offset};

#[derive(Debug, Clone)]
pub(crate) struct CiWorkflow {
    pub(crate) jobs: Vec<CiJob>,
}

#[derive(Debug, Clone)]
pub(crate) struct CiJob {
    pub(crate) id: String,
    pub(crate) line: usize,
    pub(crate) steps: Vec<CiWorkflowStep>,
}

#[derive(Debug, Clone)]
pub(crate) struct CiWorkflowStep {
    pub(crate) index: usize,
    pub(crate) name: String,
    pub(crate) line: usize,
    pub(crate) uses: Option<String>,
    pub(crate) commands: Vec<CiRunStep>,
    pub(crate) body: String,
}

pub(crate) fn ci_workflow(text: &str) -> Option<CiWorkflow> {
    let lines = text.lines().collect::<Vec<_>>();
    let (jobs_index, jobs_indent) = lines.iter().enumerate().find_map(|(index, line)| {
        let indent = leading_whitespace_count(line);
        (line.trim() == "jobs:").then_some((index, indent))
    })?;
    let mut job_starts = Vec::new();
    for (index, line) in lines.iter().enumerate().skip(jobs_index + 1) {
        let indent = leading_whitespace_count(line);
        if !line.trim().is_empty() && indent <= jobs_indent {
            break;
        }
        if indent == jobs_indent + 2
            && let Some(id) = mapping_key(line)
        {
            job_starts.push((index, id));
        }
    }
    let mut jobs = Vec::new();
    for (position, (start, id)) in job_starts.iter().enumerate() {
        let end = job_starts
            .get(position + 1)
            .map(|(index, _)| *index)
            .unwrap_or_else(|| job_block_end(&lines, *start, jobs_indent));
        jobs.push(CiJob {
            id: id.clone(),
            line: start + 1,
            steps: workflow_steps(&lines, *start, end, jobs_indent + 2),
        });
    }
    Some(CiWorkflow { jobs })
}

pub(crate) fn ci_workflow_name(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        if leading_whitespace_count(line) != 0 {
            return None;
        }
        let value = line.trim().strip_prefix("name:")?.trim();
        (!value.is_empty()).then(|| trim_yaml_scalar(value))
    })
}

fn workflow_steps(
    lines: &[&str],
    job_start: usize,
    job_end: usize,
    job_indent: usize,
) -> Vec<CiWorkflowStep> {
    let Some((steps_index, steps_indent)) = lines[job_start + 1..job_end]
        .iter()
        .enumerate()
        .find_map(|(offset, line)| {
            let indent = leading_whitespace_count(line);
            (indent > job_indent && line.trim() == "steps:")
                .then_some((job_start + 1 + offset, indent))
        })
    else {
        return Vec::new();
    };
    let mut starts = Vec::new();
    for (index, line) in lines.iter().enumerate().take(job_end).skip(steps_index + 1) {
        let indent = leading_whitespace_count(line);
        if !line.trim().is_empty() && indent <= steps_indent {
            break;
        }
        if indent == steps_indent + 2 && line.trim_start().starts_with("- ") {
            starts.push(index);
        }
    }
    starts
        .iter()
        .enumerate()
        .map(|(position, start)| {
            let end = starts.get(position + 1).copied().unwrap_or(job_end);
            workflow_step(lines, *start, end, position + 1)
        })
        .collect()
}

fn workflow_step(lines: &[&str], start: usize, end: usize, index: usize) -> CiWorkflowStep {
    let body = lines[start..end].join("\n");
    let name = step_field(lines, start, end, "name")
        .or_else(|| step_field(lines, start, end, "uses"))
        .or_else(|| {
            ci_run_steps_with_offset(&body, start)
                .first()
                .map(|step| bounded_label(&step.command))
        })
        .unwrap_or_else(|| format!("step-{index}"));
    CiWorkflowStep {
        index,
        name,
        line: start + 1,
        uses: step_field(lines, start, end, "uses"),
        commands: ci_run_steps_with_offset(&body, start),
        body,
    }
}

fn step_field(lines: &[&str], start: usize, end: usize, key: &str) -> Option<String> {
    let step_indent = leading_whitespace_count(lines[start]);
    for (offset, line) in lines[start..end].iter().enumerate() {
        let indent = leading_whitespace_count(line);
        if offset > 0 && indent > step_indent + 2 {
            continue;
        }
        let trimmed = line
            .trim_start()
            .strip_prefix("- ")
            .unwrap_or_else(|| line.trim_start());
        let Some(value) = trimmed.strip_prefix(&format!("{key}:")) else {
            continue;
        };
        let value = value.trim();
        if !value.is_empty() {
            return Some(trim_yaml_scalar(value));
        }
    }
    None
}

fn mapping_key(line: &str) -> Option<String> {
    let key = line.trim().strip_suffix(':')?.trim();
    if key.is_empty()
        || key.contains(char::is_whitespace)
        || !key
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
    {
        return None;
    }
    Some(key.to_string())
}

fn job_block_end(lines: &[&str], start: usize, jobs_indent: usize) -> usize {
    lines
        .iter()
        .enumerate()
        .skip(start + 1)
        .find_map(|(index, line)| {
            (!line.trim().is_empty() && leading_whitespace_count(line) <= jobs_indent)
                .then_some(index)
        })
        .unwrap_or(lines.len())
}

fn bounded_label(value: &str) -> String {
    let mut label = value.trim().chars().take(72).collect::<String>();
    if value.trim().chars().count() > 72 {
        label.push('…');
    }
    label
}

fn trim_yaml_scalar(value: &str) -> String {
    let value = strip_inline_comment(value);
    value
        .trim()
        .trim_matches(|ch| matches!(ch, '\'' | '"'))
        .to_string()
}

fn strip_inline_comment(value: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;
    let mut previous_whitespace = false;
    for (index, ch) in value.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double && previous_whitespace => return &value[..index],
            _ => {}
        }
        previous_whitespace = ch.is_whitespace();
    }
    value
}

fn leading_whitespace_count(line: &str) -> usize {
    line.chars().take_while(|ch| ch.is_whitespace()).count()
}

// Responsibility: static-proof-process-invocations
use crate::map::{proof_wiring_expand_for_proof, proof_wiring_fact};
use crate::model::{EvidenceLocation, Project, ProofSurface, ProofWiringFact};

pub(crate) fn process_invocation_facts(
    project: &Project,
    proof: &ProofSurface,
    selector: &str,
) -> Vec<ProofWiringFact> {
    let mut facts = Vec::new();
    if let Some(path) = proof.path.as_deref()
        && let Some(text) = project.read_indexed_text(path)
    {
        facts.extend(python_process_facts(path, &text, proof, selector));
    }
    if let Some(command) = proof.command.as_deref() {
        if let Some((runner, target)) = static_process_command(command) {
            facts.push(wired_process_fact(
                command,
                &target,
                format!("static `{runner}` process target is visible in the verification command"),
                proof.locations.clone(),
                proof_wiring_expand_for_proof(selector, proof),
            ));
        }
        facts.extend(make_like_recipe_process_facts(
            project, command, proof, selector,
        ));
    }
    facts
}

fn python_process_facts(
    path: &str,
    text: &str,
    proof: &ProofSurface,
    selector: &str,
) -> Vec<ProofWiringFact> {
    let mut facts = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let Some(call) = python_process_call(line) else {
            continue;
        };
        let location = vec![EvidenceLocation::line(
            path,
            index + 1,
            "process_invocation",
        )];
        let literals = quoted_literals(call);
        if let Some((runner, target)) = static_process_argv(&literals) {
            facts.push(wired_process_fact(
                path,
                &target,
                format!("static subprocess argv invokes `{runner} {target}`"),
                location,
                proof_wiring_expand_for_proof(selector, proof),
            ));
        } else {
            facts.push(proof_wiring_fact(
                ("invokes_process", "unknown"),
                path.to_string(),
                Some(path.to_string()),
                "subprocess invocation uses an argv shape whose external target was not statically resolved",
                "the external verification boundary remains explicit; codemap did not execute or infer it",
                location,
                proof_wiring_expand_for_proof(selector, proof),
            ));
        }
        if facts.len() >= 4 {
            break;
        }
    }
    facts
}

fn make_like_recipe_process_facts(
    project: &Project,
    command: &str,
    proof: &ProofSurface,
    selector: &str,
) -> Vec<ProofWiringFact> {
    let Some((runner, target)) = make_like_target(command) else {
        return Vec::new();
    };
    let Some(script) = project
        .scripts
        .iter()
        .find(|script| script.name == target && script.command.starts_with(runner))
    else {
        return Vec::new();
    };
    let Some(path) = script.path.as_deref() else {
        return Vec::new();
    };
    let Some(text) = project.read_indexed_text(path) else {
        return Vec::new();
    };
    let start = script.line_start.unwrap_or(1);
    let mut facts = Vec::new();
    for (index, line) in text.lines().enumerate().skip(start) {
        if !line.starts_with(char::is_whitespace) {
            break;
        }
        let recipe = line.trim_start_matches([' ', '\t', '@', '-', '+']);
        if let Some((process_runner, process_target)) = static_process_command(recipe) {
            facts.push(wired_process_fact(
                command,
                &process_target,
                format!("{runner} target `{target}` invokes `{process_runner} {process_target}`"),
                vec![EvidenceLocation::line(path, index + 1, "script_recipe")],
                proof_wiring_expand_for_proof(selector, proof),
            ));
        }
    }
    facts
}

fn python_process_call(line: &str) -> Option<&str> {
    [
        "subprocess.run(",
        "subprocess.check_call(",
        "subprocess.check_output(",
        "subprocess.Popen(",
    ]
    .into_iter()
    .find_map(|needle| line.split_once(needle).map(|(_, tail)| tail))
}

fn quoted_literals(text: &str) -> Vec<String> {
    let mut values = Vec::new();
    let mut quote = None;
    let mut current = String::new();
    let mut escaped = false;
    for ch in text.chars() {
        if let Some(active) = quote {
            if escaped {
                current.push(ch);
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == active {
                values.push(std::mem::take(&mut current));
                quote = None;
            } else {
                current.push(ch);
            }
        } else if matches!(ch, '\'' | '"') {
            quote = Some(ch);
        }
    }
    values
}

fn static_process_argv(literals: &[String]) -> Option<(String, String)> {
    if literals.len() < 2 || !known_process_runner(&literals[0]) {
        return None;
    }
    path_like_target(&literals[1]).then(|| (literals[0].clone(), literals[1].clone()))
}

fn static_process_command(command: &str) -> Option<(String, String)> {
    if command.contains([';', '|', '&', '`', '$', '\n', '\r']) {
        return None;
    }
    let tokens = command.split_whitespace().collect::<Vec<_>>();
    let runner = tokens.first()?.trim_matches(['\'', '"']);
    if !known_process_runner(runner) {
        return None;
    }
    let target = tokens
        .iter()
        .skip(1)
        .map(|token| token.trim_matches(['\'', '"', ',', ')', ']']))
        .find(|token| !token.starts_with('-') && path_like_target(token))?;
    Some((runner.to_string(), target.to_string()))
}

fn make_like_target(command: &str) -> Option<(&str, &str)> {
    let mut tokens = command.split_whitespace();
    let runner = tokens.next()?;
    matches!(runner, "make" | "just").then_some((runner, tokens.next()?))
}

fn known_process_runner(runner: &str) -> bool {
    matches!(
        runner.rsplit('/').next().unwrap_or(runner),
        "python" | "python3" | "node" | "bash" | "sh" | "ruby"
    )
}

fn path_like_target(target: &str) -> bool {
    target.contains('/')
        || target.ends_with(".py")
        || target.ends_with(".js")
        || target.ends_with(".mjs")
        || target.ends_with(".sh")
        || target.ends_with(".rb")
}

fn wired_process_fact(
    subject: &str,
    target: &str,
    evidence: String,
    locations: Vec<EvidenceLocation>,
    expand: Option<String>,
) -> ProofWiringFact {
    proof_wiring_fact(
        ("invokes_process", "wired"),
        subject.to_string(),
        Some(target.to_string()),
        evidence,
        "process invocation is structurally observed; execution, correctness, and sufficiency are not claimed",
        locations,
        expand,
    )
}

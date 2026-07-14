// Responsibility: python-route-decorators
use crate::map::{quoted_literal_at, route_like_receiver, static_route_methods};
use crate::model::{EvidenceLocation, EvidenceStrength, RuntimeRoute};

pub(crate) fn python_route_decorators(
    rel: &str,
    line: &str,
    line_number: usize,
) -> Vec<RuntimeRoute> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with('@') {
        return Vec::new();
    }
    static_route_methods()
        .iter()
        .filter_map(|method| {
            let call = format!(".{method}(");
            let start = trimmed.find(&call)?;
            let receiver_prefix = &trimmed[..start];
            if !python_decorator_receiver_is_local(receiver_prefix)
                || !route_like_receiver(receiver_prefix)
            {
                return None;
            }
            let path = quoted_literal_at(trimmed[start + call.len()..].trim_start())?;
            Some(RuntimeRoute {
                method: Some(method.to_ascii_uppercase()),
                path,
                file: rel.to_string(),
                handler_symbol: None,
                evidence: "python_route_decorator".to_string(),
                strength: EvidenceStrength::High,
                locations: vec![EvidenceLocation::line(rel, line_number, "route_decorator")],
            })
        })
        .collect()
}

fn python_decorator_receiver_is_local(prefix: &str) -> bool {
    !prefix.chars().any(|ch| matches!(ch, '(' | '[' | '{' | ','))
}

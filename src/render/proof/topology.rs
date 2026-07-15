// Responsibility: render-observed-verification-topology
use crate::model::VerificationTopology;
use crate::render::code;

pub(crate) fn verification_topology_section(topology: &VerificationTopology) {
    let horizon = &topology.horizon;
    let shown = topology.direct.len()
        + topology.mediated.len()
        + topology.runnable.len()
        + topology.soft_related.len()
        + topology.support.len()
        + topology.missing_link.len()
        + topology.unknown_external.len();
    if shown == 0 && horizon.status != "open" {
        return;
    }
    println!("\n## Verification Topology\n");
    println!(
        "- direct: `{}`; mediated: `{}`; runnable: `{}`; soft related: `{}`",
        topology.direct.len(),
        topology.mediated.len(),
        topology.runnable.len(),
        topology.soft_related.len()
    );
    println!(
        "- support: `{}`; missing link: `{}`; unknown external: `{}`",
        topology.support.len(),
        topology.missing_link.len(),
        topology.unknown_external.len()
    );
    println!(
        "- coverage horizon: `{}`; observed: `{}`; shown: `{}`; hidden: `{}`",
        horizon.status, horizon.observed, horizon.shown, horizon.hidden
    );
    if let Some(relation) = topology.mediated.first() {
        println!("- mediated path: {}", render_path(&relation.path));
    }
    if let Some(relation) = topology
        .runnable
        .iter()
        .find(|relation| relation.relation == "invokes_process")
    {
        println!("- invokes process: {}", render_path(&relation.path));
    }
    if !topology.soft_related.is_empty() {
        println!("- soft related surfaces stay separate from direct verification.");
    }
    println!("- observed relationships do not claim execution, correctness, or sufficiency.");
}

fn render_path(path: &[String]) -> String {
    path.iter()
        .map(|part| code(part))
        .collect::<Vec<_>>()
        .join(" -> ")
}

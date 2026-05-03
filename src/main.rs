mod cache;
mod cli;
mod evidence;
mod map;
mod model;
mod proof_classification;
mod render;
mod repo;

fn main() -> anyhow::Result<()> {
    cli::run()
}

mod cache;
mod cli;
mod map;
mod model;
mod render;
mod repo;

fn main() -> anyhow::Result<()> {
    cli::run()
}

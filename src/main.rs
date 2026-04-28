mod cache;
mod cli;
mod model;
mod render;
mod repo;
mod route;

fn main() -> anyhow::Result<()> {
    cli::run()
}

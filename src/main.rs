mod cli;
mod render;
mod repo;

fn main() -> anyhow::Result<()> {
    cli::run()
}

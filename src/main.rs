mod cache;
mod cli;
mod evidence;
mod map;
mod model;
mod proof_classification;
mod render;
mod repo;

fn main() -> std::process::ExitCode {
    cli::main_exit()
}

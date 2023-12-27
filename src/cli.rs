use std::path::PathBuf;
use clap::Parser;

#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    version = env!("CARGO_PKG_VERSION"),
    author = env!("CARGO_PKG_AUTHORS"),
    about = env!("CARGO_PKG_DESCRIPTION"),
)]
pub(crate) struct Args {

    #[arg(short = 'o', long)]
    pub output: Option<PathBuf>,

    #[arg(long)]
    pub mapping_file: Option<PathBuf>,

    #[arg(long)]
    pub errors_file: Option<PathBuf>,

    #[arg(long, default_value = "false")]
    pub clean_paths: bool,

    #[arg(long)]
    pub remove_prefix: Option<String>,

    #[arg(long, default_value = "()[]{}-+*=&@!?'#$%^~^´`:,;<>|\"\\")]
    pub filter_chars: String,

    #[arg()]
    pub input: PathBuf,
}
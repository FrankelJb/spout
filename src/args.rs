use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, default_value = "0.0.0.0")]
    pub local_host: String,

    #[arg(short, long)]
    pub ports: Vec<u16>,

    #[arg(short, long)]
    pub remote_host: String,
}

use clap::Parser;

#[derive(Parser)]
struct Arguments {
    mode: String,
}

fn main() {
    let args = Arguments::parse();

    println!("Hello, world! Mode: {}", args.mode);
}

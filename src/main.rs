use clap::{arg, Parser};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Run alt fib constraint layout
    #[arg(long, default_value_t = false)]
    run_alt: bool,

    /// Create plot of circuit layout
    #[arg(long, default_value_t = true)]
    plot: bool,

    #[arg(long, default_value_t = 180)]
    num_steps: usize,
}

fn main() {
    let args = Args::parse();

    if args.run_alt {
        halo2_fib::fib_2::run(args.plot, args.num_steps);
    } else {
        halo2_fib::fib::run(args.plot, args.num_steps);
    }
}

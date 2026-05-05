#[derive(clap::Parser)]
pub struct Args {
    #[arg(long)]
    pub seed: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn cli_parses_seed_flag() {
        let args = Args::try_parse_from(["fungai", "--seed", "999"]).unwrap();
        assert_eq!(args.seed, Some(999));
    }

    #[test]
    fn cli_no_seed_flag_yields_none() {
        let args = Args::try_parse_from(["fungai"]).unwrap();
        assert_eq!(args.seed, None);
    }
}

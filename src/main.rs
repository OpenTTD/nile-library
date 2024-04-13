use clap::Parser;

mod validate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    base: String,
    case: String,
    translation: String,

    #[clap(short, long)]
    cases: Vec<String>,
    #[clap(short, long)]
    genders: Vec<String>,
    #[clap(short, long, default_value_t = 1)]
    plural_count: u32,
}

fn main() {
    let args = Args::parse();
    let config = validate::LanguageConfig {
        cases: args.cases,
        genders: args.genders,
        plural_count: args.plural_count,
    };

    let result = validate::validate(config, args.base, args.case, args.translation);

    if let Some(error) = result {
        println!("Validation failed: {}", error.message);
    } else {
        println!("Validation succeeded");
    }
}

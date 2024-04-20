use clap::Parser;

mod commands;
mod parser;
mod validate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    base: String,
    case: String,
    translation: String,

    #[clap(short, long, default_value_t = String::from("openttd"))]
    dialect: String,
    #[clap(short, long)]
    cases: Vec<String>,
    #[clap(short, long)]
    genders: Vec<String>,
    #[clap(short, long, default_value_t = 1)]
    plural_count: usize,
}

fn main() {
    let args = Args::parse();
    let config = validate::LanguageConfig {
        dialect: args.dialect,
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

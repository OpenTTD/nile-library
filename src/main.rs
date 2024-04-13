use clap::Parser;

mod validate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    base: String,
    case: String,
    translation: String,
}

fn main() {
    let args = Args::parse();

    let result = validate::validate(args.base, args.case, args.translation);

    if let Some(error) = result {
        println!("Validation failed: {}", error);
    } else {
        println!("Validation succeeded");
    }
}

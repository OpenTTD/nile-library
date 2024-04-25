use clap::Parser;

mod commands;
mod parser;
mod validate;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    base: String,
    translation: Option<String>,
    case: Option<String>,

    #[clap(short, long, default_value_t = String::from("openttd"))]
    dialect: String,
    #[clap(short, long)]
    cases: Vec<String>,
    #[clap(short, long)]
    genders: Vec<String>,
    #[clap(short, long, default_value_t = 2)]
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

    let result = match args.translation {
        Some(translation) => validate::validate_translation(
            config,
            args.base,
            args.case.unwrap_or(String::from("")),
            translation,
        ),
        None => validate::validate_base(config, args.base),
    };

    for err in &result.errors {
        let sev = if err.critical { "ERROR" } else { "WARNING" };
        let pos_begin = err
            .pos_begin
            .map_or(String::new(), |p| format!(" at byte {}", p));
        let pos_end = err
            .pos_end
            .map_or(String::new(), |p| format!(" to {}", p));
        println!("{}{}{}: {}", sev, pos_begin, pos_end, err.message);
        if let Some(hint) = &err.suggestion {
            println!("HINT: {}", hint);
        }
    }

    if let Some(normalized) = result.normalized {
        println!("NORMALIZED:{}", normalized);
    }
}

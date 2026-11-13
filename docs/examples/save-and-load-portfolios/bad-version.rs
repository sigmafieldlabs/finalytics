use finalytics::prelude::*;

fn main() {
    let text = concat!(
        "{\"format\":\"finalytics.portfolio\",\"format_version\":2,",
        "\"portfolio\":{}}",
    );
    match Portfolio::from_json(text) {
        Err(Error::Validation(report)) => {
            for issue in report.issues() {
                println!("{} | {}", issue.path, issue.message);
            }
        }
        _ => println!("Unexpected result"),
    }
}

use std::str::FromStr;

use log4rs::{
    append::file::FileAppender,
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    Config,
};

pub fn setup() {

    /*let file = File::create("target/app.log").unwrap();
    env_logger::Builder::new()
        .target(env_logger::Target::Pipe(Box::new(file)))
        .init();*/
    let level = std::env::var("RUST_LOG").unwrap_or("INFO".to_string());
    let level = log::LevelFilter::from_str(&level).unwrap_or(log::LevelFilter::Info);
    let log_file = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} {l} {m}\n",
        )))
        .build("target/logs.out")
        .unwrap();
    let config = Config::builder()
        .appender(Appender::builder().build("log_file", Box::new(log_file)))
        .build(
            Root::builder()
                .appender("log_file")
                .build(level),
        )
        .unwrap();

    let _handle = log4rs::init_config(config);
}

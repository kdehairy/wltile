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
                .build(log::LevelFilter::Trace),
        )
        .unwrap();

    let _handle = log4rs::init_config(config);
}

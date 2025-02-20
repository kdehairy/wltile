mod logging;
mod wlr_client;

use logging::setup_logging;

use log::info;


fn main() {
    setup_logging();
    info!("===Started===");
    let mut client = wlr_client::Client::new();
    let _res = client.connect();
}

use std::time::Duration;

use crate::wlr_client::{self, errors::ClientError};

/// Upper bound on how long to wait for every surface to be committed to the
/// compositor before blocking for user input. Committing is normally a handful
/// of milliseconds; this only guards against a compositor that never configures
/// a surface, turning a would-be hang into a best-effort continue.
const PRESENT_TIMEOUT: Duration = Duration::from_secs(2);

#[allow(clippy::print_stdout)]
pub(crate) fn exec(client: &mut wlr_client::Client) -> Result<(), ClientError> {
    let heads = client.configurations_read_lock().read().heads().unwrap();
    let heads = heads.heads();
    let mut display_server = client.new_display_server()?;
    for head in heads {
        display_server.write(head.name(), Some(head))?;
    }
    // Ensure the surfaces are actually committed before we go on to block for
    // input; otherwise presentation races the background dispatch loop.
    display_server.wait_until_presented(PRESENT_TIMEOUT);
    let input_server = client.get_input_server()?;
    input_server.wait_for_user_input();
    Ok(())
}

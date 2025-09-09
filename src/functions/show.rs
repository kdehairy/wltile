use crate::wlr_client::{self, errors::ClientError};

#[allow(clippy::print_stdout)]
pub(crate) fn exec(client: &mut wlr_client::Client) -> Result<(), ClientError> {
    let heads = client.configurations_read_lock().read().heads().unwrap();
    let heads = heads.heads();
    let mut display_server = client.new_display_server()?;
    for head in heads {
        display_server.write(head.name(), Some(head))?;
    }
    let input_server = client.get_input_server()?;
    input_server.wait_for_user_input();
    Ok(())
}

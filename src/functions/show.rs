use crate::wlr_client::{self, errors::ClientError, output::wlr_head::OutputHead};

#[allow(clippy::print_stdout)]
pub(crate) fn exec(client: &mut wlr_client::Client) -> Result<(), ClientError> {
    let output_heads: Vec<OutputHead> = client
        .configurations()
        .heads()
        .unwrap()
        .heads()
        .iter()
        .map(|v| v.output_head().clone())
        .collect();
    let display_server = client.get_display_server()?;
    for head in output_heads {
        display_server.write(head.name(), Some(&head))?;
        //thread::sleep(Duration::from_secs(1));
    }
    let input_server = client.get_input_server()?;
    input_server.wait_for_user_input();
    Ok(())
}

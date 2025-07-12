use std::io::{stdin, stdout, Write};

use crate::wlr_client::{self, errors::ClientError, wlr_head::OutputHead};

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
    }
    //display_server.write("Hello", None)?;
    //client.render_text("HDMI-2", None)?;
    print!("Press Enter to continue...");
    let _ = stdout().flush();
    let mut s = String::new();
    let _ = stdin().read_line(&mut s);
    Ok(())
}

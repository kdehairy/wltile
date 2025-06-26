use crate::wlr_client::{self, errors::ClientError, point::Point};

pub(crate) fn exec(client: &mut wlr_client::Client) -> Result<(), ClientError> {
    client.render_text("Hello!", Point(0, 0))
}

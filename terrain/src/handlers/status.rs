use anyhow::Result;
use terrainium_lib::pb;
use terrainium_lib::socket::Socket;

/// Returns status for specified terrain and session
pub async fn handle(
    _json: bool,
    _terrain_name: String,
    _session_id: Option<String>,
    _recent: Option<u32>,
    _client: Option<Box<dyn Socket<pb::Response, pb::Request>>>,
) -> Result<()> {
    todo!()
}

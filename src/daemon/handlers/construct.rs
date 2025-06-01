use crate::daemon::handlers::RequestHandler;
use crate::daemon::types::context::DaemonContext;
use prost_types::Any;

pub(crate) struct ConstructHandler;

impl RequestHandler for ConstructHandler {
    async fn handle(request: Any, context: DaemonContext) -> Any {
        todo!()
    }
}

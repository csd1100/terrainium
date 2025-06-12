use crate::common::types::pb;

#[derive(Debug, PartialEq)]
pub enum ProtoRequest {
    Activate(pb::Activate),
    Deactivate(pb::Deactivate),
    Execute(pb::Execute),
    Status(pb::StatusRequest),
}

pub enum ProtoResponse {
    Success,
    Status(Box<pb::StatusResponse>),
}

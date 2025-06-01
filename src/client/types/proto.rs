use crate::common::types::pb;

#[derive(Debug)]
pub enum ProtoRequest {
    Activate(pb::Activate),
    Deactivate(pb::Deactivate),
    Construct(pb::Construct),
    Destruct(pb::Destruct),
    Status(pb::StatusRequest),
}

pub enum ProtoResponse {
    Success,
    Status(pb::StatusResponse),
}

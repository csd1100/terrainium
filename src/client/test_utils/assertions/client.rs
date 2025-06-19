use crate::client::types::client::MockClient;
use crate::client::types::proto::{ProtoRequest, ProtoResponse};
use anyhow::bail;
use mockall::predicate::eq;

pub struct ExpectClient {
    request: ProtoRequest,
    expected_response: ProtoResponse,
}

impl ExpectClient {
    pub fn send(request: ProtoRequest) -> Self {
        Self {
            request,
            expected_response: ProtoResponse::Success,
        }
    }

    pub fn with_expected_response(mut self, expected_response: ProtoResponse) -> Self {
        self.expected_response = expected_response;
        self
    }

    pub fn successfully(self) -> MockClient {
        let ExpectClient {
            request,
            expected_response,
        } = self;
        let mut client = MockClient::default();
        client
            .expect_request()
            .with(eq(request))
            .return_once(move |_| Ok(expected_response));
        client
    }

    pub fn with_returning_error(self, error: &str) -> MockClient {
        let ExpectClient { request, .. } = self;
        let mut client = MockClient::default();
        let error = error.to_string();
        client
            .expect_request()
            .with(eq(request))
            .return_once(move |_| bail!(error));
        client
    }
}

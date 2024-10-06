use crate::client::types::client::Client;
use crate::common::constants::TERRAINIUMD_SOCKET;
use crate::common::types::pb;
use crate::common::types::pb::{StatusPoll, StatusRequest};
use crate::common::types::socket::Socket;
use crate::common::types::terrain_state::TerrainState;
use anyhow::{Context, Result};
use prost_types::Any;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

struct StateRenderer {
    should_quit: bool,
    state_widget: StateWidget,
}

struct StateWidget {
    terrain_name: String,
    session_id: String,
    state: Arc<RwLock<State>>,
}

struct State {
    terrain_state: TerrainState,
    last_update: String,
}

impl StateWidget {
    async fn fetch(&self) {
        loop {
            if self.should_request().await {
                self.update_status().await;
            }
        }
    }

    async fn should_request(&self) -> bool {
        let mut client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET))
            .await
            .unwrap();

        let poll_request = StatusPoll {
            session_id: self.session_id.clone(),
            terrain_name: self.terrain_name.clone(),
            last_modified: self.state.read().unwrap().last_update.clone(),
        };

        client
            .write_and_stop(Any::from_msg(&poll_request).unwrap())
            .await
            .unwrap();

        let response: Any = client.read().await.unwrap();

        let status_response: Result<pb::StatusPollResponse> = response
            .to_msg()
            .context("failed to convert status response from any");

        if let Ok(status) = status_response {
            status.is_updated
        } else {
            false
        }
    }

    async fn update_status(&self) {
        let mut client = Client::new(PathBuf::from(TERRAINIUMD_SOCKET))
            .await
            .unwrap();

        let status_request = StatusRequest {
            session_id: self.session_id.clone(),
            terrain_name: self.terrain_name.clone(),
        };

        client
            .write_and_stop(Any::from_msg(&status_request).unwrap())
            .await
            .unwrap();

        let response: Any = client.read().await.unwrap();

        let status_response: Result<pb::StatusResponse> = response
            .to_msg()
            .context("failed to convert status response from any");

        if let Ok(status) = status_response {
            let mut state = self.state.write().unwrap();
            state.last_update = status.last_modified.clone();
            state.terrain_state = status.into();
        }
    }
}

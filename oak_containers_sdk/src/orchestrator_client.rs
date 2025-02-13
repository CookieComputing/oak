//
// Copyright 2023 The Project Oak Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use anyhow::{Context, Result};
use oak_grpc::oak::containers::orchestrator_client::OrchestratorClient as GrpcOrchestratorClient;
use oak_proto_rust::oak::session::v1::EndorsedEvidence;
use tonic::transport::{Endpoint, Uri};
use tower::service_fn;

use crate::{IGNORED_ENDPOINT_URI, IPC_SOCKET};

/// Utility struct used to interface with the Orchestrator.
#[derive(Clone)]
pub struct OrchestratorClient {
    inner: GrpcOrchestratorClient<tonic::transport::channel::Channel>,
}

impl OrchestratorClient {
    pub async fn create() -> Result<Self> {
        let inner: GrpcOrchestratorClient<tonic::transport::channel::Channel> = {
            let channel = Endpoint::try_from(IGNORED_ENDPOINT_URI)
                .context("couldn't form endpoint")?
                .connect_with_connector(service_fn(move |_: Uri| {
                    tokio::net::UnixStream::connect(IPC_SOCKET)
                }))
                .await
                .context("couldn't connect to UDS socket")?;

            GrpcOrchestratorClient::new(channel)
        };
        Ok(Self { inner })
    }

    /// Retrieves the application configuration from the Orchestrator.
    /// This configuration contains settings and parameters specific to the
    /// application.
    pub async fn get_application_config(&mut self) -> Result<Vec<u8>> {
        Ok(self.inner.get_application_config(()).await?.into_inner().config)
    }

    /// Notifies the Orchestrator that the application is ready to receive
    /// requests. This should be called after the application has completed
    /// its initialization.
    pub async fn notify_app_ready(&mut self) -> Result<()> {
        self.inner.notify_app_ready(tonic::Request::new(())).await?;
        Ok(())
    }

    /// Retrieves the endorsed evidence from the Orchestrator.
    /// This evidence is used to prove the authenticity and integrity of the
    /// application.
    // TODO: b/356381841 - Remove this function once all clients start using
    // the `EndorsedEvidenceProvider`.
    pub async fn get_endorsed_evidence(&mut self) -> Result<EndorsedEvidence> {
        Ok(self.inner.get_endorsed_evidence(()).await?.into_inner())
    }
}

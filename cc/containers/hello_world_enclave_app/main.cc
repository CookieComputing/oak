/*
 * Copyright 2024 The Project Oak Authors
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
#include <memory>

#include "absl/log/check.h"
#include "absl/log/initialize.h"
#include "absl/log/log.h"
#include "absl/status/statusor.h"
#include "app_service.h"
#include "cc/containers/hello_world_enclave_app/app_service.h"
#include "cc/containers/sdk/orchestrator_client.h"
#include "grpcpp/security/server_credentials.h"
#include "grpcpp/server.h"
#include "grpcpp/server_builder.h"

using ::oak::containers::hello_world_enclave_app::EnclaveApplicationImpl;
using ::oak::containers::sdk::OrchestratorClient;

int main(int argc, char* argv[]) {
  absl::InitializeLog();
  std::clog << "Starting CC Enclave app" << std::endl;

  OrchestratorClient client;
  absl::StatusOr<std::string> application_config =
      client.GetApplicationConfig();
  QCHECK_OK(application_config);

  absl::StatusOr<oak::session::v1::EndorsedEvidence> endorsed_evidence =
      client.GetEndorsedEvidence();
  QCHECK_OK(endorsed_evidence);

  EnclaveApplicationImpl service(*application_config);

  grpc::ServerBuilder builder;
  builder.AddListeningPort("[::]:8080", grpc::InsecureServerCredentials());
  builder.RegisterService(&service);
  std::unique_ptr<grpc::Server> server(builder.BuildAndStart());
  QCHECK_OK(client.NotifyAppReady());

  std::clog << "Enclave Application is running on port 8080" << std::endl;

  server->Wait();
  return 0;
}

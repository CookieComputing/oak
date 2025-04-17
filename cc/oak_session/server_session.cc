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

#include "cc/oak_session/server_session.h"

#include <memory>
#include <optional>
#include <string>

#include "absl/status/status.h"
#include "absl/status/statusor.h"
#include "cc/ffi/bytes_bindings.h"
#include "cc/ffi/bytes_view.h"
#include "cc/ffi/error_bindings.h"
#include "cc/ffi/rust_bytes.h"
#include "cc/oak_session/config.h"
#include "cc/oak_session/oak_session_bindings.h"
#include "proto/session/session.pb.h"

namespace oak::session {

absl::StatusOr<std::unique_ptr<ServerSession>> ServerSession::Create(
    session::SessionConfig* config) {
  const bindings::ErrorOrServerSession result =
      bindings::new_server_session(config);

  if (result.error != nullptr) {
    return ffi::bindings::ErrorIntoStatus(result.error);
  }

  return absl::WrapUnique(new ServerSession(result.result));
}

absl::StatusOr<std::unique_ptr<ServerSession>> ServerSession::Create() {
  return Create(SessionConfigBuilder(session::AttestationType::kUnattested,
                                     session::HandshakeType::kNoiseNN)
                    .Build());
}

bool ServerSession::IsOpen() { return bindings::server_is_open(rust_session_); }

absl::Status ServerSession::PutIncomingMessage(
    const v1::SessionRequest& request) {
  const std::string request_bytes = request.SerializeAsString();
  ffi::bindings::Error* error = bindings::server_put_incoming_message(
      rust_session_, ffi::bindings::BytesView(request_bytes));
  return ffi::bindings::ErrorIntoStatus(error);
}

absl::StatusOr<std::optional<v1::SessionResponse>>
ServerSession::GetOutgoingMessage() {
  const ffi::bindings::ErrorOrRustBytes result =
      bindings::server_get_outgoing_message(rust_session_);
  if (result.error != nullptr) {
    return ffi::bindings::ErrorIntoStatus(result.error);
  }

  if (result.result == nullptr) {
    return std::nullopt;
  }

  v1::SessionResponse response;
  if (!response.ParseFromString(*result.result)) {
    return absl::InternalError(
        "Failed to parse GetOutoingMessage result bytes as SessionResponse");
  }

  ffi::bindings::free_rust_bytes(result.result);
  return response;
}

absl::Status ServerSession::Write(
    const v1::PlaintextMessage& unencrypted_request) {
  return Write(unencrypted_request.plaintext());
}

absl::Status ServerSession::Write(absl::string_view unencrypted_request) {
  ffi::bindings::Error* error = bindings::server_write(
      rust_session_, ffi::bindings::BytesView(unencrypted_request));

  return ffi::bindings::ErrorIntoStatus(error);
}

absl::StatusOr<std::optional<v1::PlaintextMessage>> ServerSession::Read() {
  const ffi::bindings::ErrorOrRustBytes result =
      bindings::server_read(rust_session_);
  if (result.error != nullptr) {
    return ffi::bindings::ErrorIntoStatus(result.error);
  }

  if (result.result == nullptr) {
    return std::nullopt;
  }

  v1::PlaintextMessage plaintext_message_result;
  plaintext_message_result.set_plaintext(*result.result);

  ffi::bindings::free_rust_bytes(result.result);
  return plaintext_message_result;
}

absl::StatusOr<std::optional<ffi::RustBytes>> ServerSession::ReadToRustBytes() {
  const ffi::bindings::ErrorOrRustBytes result =
      bindings::server_read(rust_session_);
  if (result.error != nullptr) {
    return ffi::bindings::ErrorIntoStatus(result.error);
  }

  if (result.result == nullptr) {
    return std::nullopt;
  }

  return ffi::RustBytes(result.result);
}

ServerSession::~ServerSession() {
  bindings::free_server_session(rust_session_);
}

}  // namespace oak::session

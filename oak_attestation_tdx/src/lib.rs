//
// Copyright 2025 The Project Oak Authors
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
//

extern crate alloc;

use alloc::vec::Vec;

use anyhow::Context;
#[allow(deprecated)]
use oak_attestation::ApplicationKeysAttester;
use oak_attestation::LayerData;
use oak_attestation_types::{
    attester::Attester,
    util::{encode_length_delimited_proto, try_decode_length_delimited_proto, Serializable},
};
use oak_proto_rust::oak::attestation::v1::{DiceData, EventLog, Evidence};
use p256::ecdsa::VerifyingKey;
use sha2::{Digest, Sha384};

/// Attester that uses Runtime Measurement Registers (RTMRs) to provide
/// integrity for the event log entries.
pub struct RtmrAttester {
    evidence: Evidence,
}

// TODO: b/368030563 - Remove this implementation once all client library
// instances use the applications keys from the event log.
#[allow(deprecated)]
impl ApplicationKeysAttester for RtmrAttester {
    // Not used for RTMRs.
    fn add_application_keys(
        self,
        _layer_data: LayerData,
        _kem_public_key: &[u8],
        _verifying_key: &VerifyingKey,
        _group_kem_public_key: Option<&[u8]>,
        _group_verifying_key: Option<&VerifyingKey>,
    ) -> anyhow::Result<Evidence> {
        anyhow::bail!("Not implemented");
    }
}

impl Attester for RtmrAttester {
    fn extend(&mut self, encoded_event: &[u8]) -> anyhow::Result<()> {
        self.evidence
            .event_log
            .get_or_insert_with(EventLog::default)
            .encoded_events
            .push(encoded_event.to_vec());
        self.evidence.root_layer = None;

        let digest = Sha384::digest(encoded_event);
        // We extend RTMR2 for all event log entries.
        //
        // The `oak_configfs_tsm`` API is async but the Attester trait is not, so we
        // have to find the current async runtime handle. `Handle::current` will
        // panic if this is not run inside a tokio runtime. This should be OK,
        // since it is only used from Stage 1 and the Orchestrator which both
        // use `tokio_main`.
        tokio::runtime::Handle::current()
            .block_on(oak_configfs_tsm::extend(oak_configfs_tsm::RTMR::RTMR2, digest.into()))
            .map(|_| ())
            .context("couldn't extend RTMR2")
    }

    fn quote(&self) -> anyhow::Result<Evidence> {
        // TODO: b/380443519 - Generate attestation report if it doesn't exist.
        Ok(self.evidence.clone())
    }
}

impl Serializable for RtmrAttester {
    fn deserialize(bytes: &[u8]) -> anyhow::Result<Self> {
        // TODO: b/368023328 - Rename DiceData.
        let attestation_data: DiceData = try_decode_length_delimited_proto(bytes)
            .context("couldn't parse attestation data: {:?}")?;
        let evidence = attestation_data.evidence.ok_or_else(|| anyhow::anyhow!("no evidence"))?;
        Ok(RtmrAttester { evidence })
    }

    fn serialize(self) -> Vec<u8> {
        // TODO: b/368023328 - Rename DiceData.
        let attestation_data =
            DiceData { evidence: Some(self.evidence), certificate_authority: None };
        encode_length_delimited_proto(&attestation_data)
    }
}

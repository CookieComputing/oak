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

use env_logger::Env;
pub use log::{debug, error, info};

pub fn init_logging(enable_logging: bool) {
    if enable_logging {
        env_logger::init();
    } else {
        disable_icing_logging();
        let env = Env::default().filter_or("RUST_LOG", "off");
        env_logger::init_from_env(env);
    }
}

pub fn disable_icing_logging() {
    icing::set_logging(false);
}

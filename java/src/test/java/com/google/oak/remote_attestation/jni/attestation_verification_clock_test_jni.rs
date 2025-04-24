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

use jni::{
    objects::{JClass, JObject},
    sys::jlong,
    JNIEnv,
};
use oak_attestation_verification_types::util::Clock;
use oak_jni_attestation_verification_clock::JNIClock;

#[allow(non_snake_case)]
#[no_mangle]
fn Java_com_google_oak_remote_1attestation_AttestationVerificationClockTest_newRustJniClock(
    env: JNIEnv,
    _class: JClass,
    java_clock_impl: JObject,
) -> jlong {
    let jni_clock = JNIClock::new(&env, &java_clock_impl).expect("Failed to create clock");

    Box::into_raw(Box::new(jni_clock)) as jlong
}

#[allow(non_snake_case)]
#[no_mangle]
fn Java_com_google_oak_remote_1attestation_AttestationVerificationClockTest_rustJniClockGetTime(
    _env: JNIEnv,
    _class: JClass,
    jni_clock_ptr: jlong,
) -> jlong {
    unsafe { &*(jni_clock_ptr as *mut JNIClock) }.get_milliseconds_since_epoch() as jlong
}

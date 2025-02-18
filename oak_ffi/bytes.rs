//
// Copyright 2024 The Project Oak Authors
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

/// A basic wrapper around Rust or C-provided bytes of known length.
///
/// This structure can be passed back and forth between Rust and C code.
/// Functions that use the type should explain the lifetime expectations for the
/// type.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct BytesView {
    data: *const u8,
    len: usize,
}

impl BytesView {
    /// Create a new instance wrapping the provided data of the specified
    /// length.
    pub fn new(data: *const u8, len: usize) -> BytesView {
        BytesView { data, len }
    }

    /// Create a new instance wrapping the provided slice.
    pub fn new_from_slice(slice: &[u8]) -> BytesView {
        BytesView::new(slice.as_ptr(), slice.len())
    }

    /// Return a `std::slice` representation of this [`BytesView`] instance.
    /// There will not be any ownership changes.
    ///
    /// # Safety
    /// The instance contains a non-null, properly aligned, valid pointer.
    pub unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.data, self.len)
    }
}

/// A basic wrapper around Rust-provided bytes of known length.
///
/// Rust code can use this to pass Rust-owned data back to C.
/// C code is responsible for releasing the bytes using `free_rust_bytes`.
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct RustBytes {
    data: *const u8,
    len: usize,
}

impl RustBytes {
    /// Create a new [`RustBytes`] instance from the provided [`Box<[u8]>`].
    /// Ownership of the box will be released, so the memory will need
    /// to be freed later with a call to [`free_rust_bytes`].
    pub fn new(bytes: Box<[u8]>) -> RustBytes {
        let raw_bytes_ptr = Box::into_raw(bytes);
        RustBytes { data: raw_bytes_ptr as *const u8, len: raw_bytes_ptr.len() }
    }

    /// Return a `std::slice` representation of this [`RustBytes`] instance.
    /// There will not be any ownership changes.
    ///
    /// # Safety
    /// The instance contains a non-null, properly aligned, valid pointer.
    pub unsafe fn as_slice(&self) -> &[u8] {
        std::slice::from_raw_parts(self.data, self.len)
    }

    /// Return a [`BytesView`] containing the Rust bytes.
    pub fn as_bytes_view(&self) -> BytesView {
        BytesView::new(self.data, self.len)
    }
}

///  Return ownership of the [`RustBytes`] pointer back to Rust, where
/// it will be  dropped and all related memory released, including the allocated
/// contents.
///
/// Note: if you have a [`RustBytes`] structure, but not a poiner to it, use
/// [`free_rust_bytes_contents`]` instead.
///
/// # Safety
///
///  * The provided [`Bytes`] is a valid, still allocated instance, containing
///    valid, allocated bytes.
///  * The instance should not be used anymore after calling this function.
#[no_mangle]
pub unsafe extern "C" fn free_rust_bytes(bytes: *const RustBytes) {
    let bytes_boxed = Box::from_raw(bytes as *mut RustBytes);
    free_rust_bytes_contents(*bytes);
    drop(bytes_boxed)
}

/// Release the rust memory owned by the provided Bytes struct.
///
/// # Safety
///
///  * The provided [`Bytes`] is a valid, still allocated instance, containing
///    valid, allocated bytes. It should not be used anymore after calling this
///    function.
#[no_mangle]
pub unsafe extern "C" fn free_rust_bytes_contents(bytes: RustBytes) {
    drop(Box::from_raw(std::slice::from_raw_parts_mut(bytes.data as *mut u8, bytes.len)))
}

/*
 * Copyright (C) 2020 The Android Open Source Project
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! This crate provides a FUSE-based, non-generic filesystem that I/O is authenticated. This
//! filesystem assumes the storage layer is not trusted, e.g. file is provided by an untrusted VM,
//! and the content can't be simply trusted. The filesystem can use its public key to verify a
//! (read-only) file against its associated fs-verity signature by a trusted party. With the Merkle
//! tree, each read of file block can be verified individually.
//!
//! The implementation is not finished.

mod reader;

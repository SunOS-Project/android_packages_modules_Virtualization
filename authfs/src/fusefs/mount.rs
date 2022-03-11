/*
 * Copyright (C) 2021 The Android Open Source Project
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

use fuse::mount::MountOption;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use std::path::Path;

use super::AuthFs;

/// Maximum bytes in the write transaction to the FUSE device. This limits the maximum buffer
/// size in a read request (including FUSE protocol overhead) that the filesystem writes to.
pub const MAX_WRITE_BYTES: u32 = 65536;

/// Maximum bytes in a read operation.
/// TODO(victorhsieh): This option is deprecated by FUSE. Figure out if we can remove this.
const MAX_READ_BYTES: u32 = 65536;

/// Mount and start the FUSE instance to handle messages. This requires CAP_SYS_ADMIN.
pub fn mount_and_enter_message_loop(
    authfs: AuthFs,
    mountpoint: &Path,
    extra_options: &Option<String>,
) -> Result<(), fuse::Error> {
    let dev_fuse = OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/fuse")
        .expect("Failed to open /dev/fuse");

    let mut mount_options = vec![
        MountOption::FD(dev_fuse.as_raw_fd()),
        MountOption::RootMode(libc::S_IFDIR | libc::S_IXUSR | libc::S_IXGRP | libc::S_IXOTH),
        MountOption::AllowOther,
        MountOption::UserId(0),
        MountOption::GroupId(0),
        MountOption::MaxRead(MAX_READ_BYTES),
    ];
    if let Some(value) = extra_options {
        mount_options.push(MountOption::Extra(value));
    }

    fuse::mount(
        mountpoint,
        "authfs",
        libc::MS_NOSUID | libc::MS_NODEV | libc::MS_NOEXEC,
        &mount_options,
    )
    .expect("Failed to mount fuse");

    fuse::worker::start_message_loop(dev_fuse, MAX_WRITE_BYTES, MAX_READ_BYTES, authfs)
}

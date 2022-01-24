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

//! Implementation of IIsolatedCompilationService, called from system server when compilation is
//! desired.

use crate::compilation_task::CompilationTask;
use crate::fd_server_helper::FdServerConfig;
use crate::instance_manager::InstanceManager;
use crate::util::to_binder_result;
use android_system_composd::aidl::android::system::composd::{
    ICompilationTask::{BnCompilationTask, ICompilationTask},
    ICompilationTaskCallback::ICompilationTaskCallback,
    IIsolatedCompilationService::{BnIsolatedCompilationService, IIsolatedCompilationService},
};
use android_system_composd::binder::{
    self, BinderFeatures, ExceptionCode, Interface, Status, Strong, ThreadState,
};
use anyhow::{Context, Result};
use rustutils::{system_properties, users::AID_ROOT, users::AID_SYSTEM};
use std::fs::{File, OpenOptions};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;
use std::path::Path;
use std::sync::Arc;

pub struct IsolatedCompilationService {
    instance_manager: Arc<InstanceManager>,
}

pub fn new_binder(
    instance_manager: Arc<InstanceManager>,
) -> Strong<dyn IIsolatedCompilationService> {
    let service = IsolatedCompilationService { instance_manager };
    BnIsolatedCompilationService::new_binder(service, BinderFeatures::default())
}

impl Interface for IsolatedCompilationService {}

impl IIsolatedCompilationService for IsolatedCompilationService {
    fn startStagedApexCompile(
        &self,
        callback: &Strong<dyn ICompilationTaskCallback>,
    ) -> binder::Result<Strong<dyn ICompilationTask>> {
        check_permissions()?;
        to_binder_result(self.do_start_staged_apex_compile(callback))
    }

    fn startTestCompile(
        &self,
        callback: &Strong<dyn ICompilationTaskCallback>,
    ) -> binder::Result<Strong<dyn ICompilationTask>> {
        check_permissions()?;
        to_binder_result(self.do_start_test_compile(callback))
    }

    fn startTestOdrefresh(&self) -> binder::Result<i8> {
        check_permissions()?;
        to_binder_result(self.do_odrefresh_for_test())
    }
}

impl IsolatedCompilationService {
    fn do_start_staged_apex_compile(
        &self,
        callback: &Strong<dyn ICompilationTaskCallback>,
    ) -> Result<Strong<dyn ICompilationTask>> {
        // TODO: Try to start the current instance with staged APEXes to see if it works?
        let comp_os = self.instance_manager.start_pending_instance().context("Starting CompOS")?;

        let task = CompilationTask::start_staged_apex_compile(comp_os, callback)?;

        Ok(BnCompilationTask::new_binder(task, BinderFeatures::default()))
    }

    fn do_start_test_compile(
        &self,
        callback: &Strong<dyn ICompilationTaskCallback>,
    ) -> Result<Strong<dyn ICompilationTask>> {
        let comp_os = self.instance_manager.start_test_instance().context("Starting CompOS")?;

        let task = CompilationTask::start_test_compile(comp_os, callback)?;

        Ok(BnCompilationTask::new_binder(task, BinderFeatures::default()))
    }

    fn do_odrefresh_for_test(&self) -> Result<i8> {
        let compos = self
            .instance_manager
            .start_test_instance()
            .context("Starting CompOS for odrefresh test")?;

        let output_dir = open_dir(composd_native::palette_create_odrefresh_staging_directory()?)?;
        let system_dir = open_dir(Path::new("/system"))?;

        // Spawn a fd_server to serve the FDs.
        let fd_server_config = FdServerConfig {
            ro_dir_fds: vec![system_dir.as_raw_fd()],
            rw_dir_fds: vec![output_dir.as_raw_fd()],
            ..Default::default()
        };
        let fd_server_raii = fd_server_config.into_fd_server()?;

        let zygote_arch = system_properties::read("ro.zygote")?;
        let result = compos.get_service().odrefresh(
            system_dir.as_raw_fd(),
            output_dir.as_raw_fd(),
            &zygote_arch,
        );
        drop(fd_server_raii);
        Ok(result?.exitCode)
    }
}

fn check_permissions() -> binder::Result<()> {
    let calling_uid = ThreadState::get_calling_uid();
    // This should only be called by system server, or root while testing
    if calling_uid != AID_SYSTEM && calling_uid != AID_ROOT {
        Err(Status::new_exception(ExceptionCode::SECURITY, None))
    } else {
        Ok(())
    }
}

/// Returns an owned FD of the directory. It currently returns a `File` as a FD owner, but
/// it's better to use `std::os::unix::io::OwnedFd` once/if it becomes standard.
fn open_dir(path: &Path) -> Result<File> {
    OpenOptions::new()
        .custom_flags(libc::O_DIRECTORY)
        .read(true) // O_DIRECTORY can only be opened with read
        .open(path)
        .with_context(|| format!("Failed to open {:?} directory as path fd", path))
}

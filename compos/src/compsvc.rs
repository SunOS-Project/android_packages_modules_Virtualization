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

//! compsvc is a service to run compilation tasks in a PVM upon request. It is able to set up
//! file descriptors backed by authfs (via authfs_service) and pass the file descriptors to the
//! actual compiler.

use anyhow::Result;
use binder_common::new_binder_exception;
use log::warn;
use std::default::Default;
use std::env;
use std::path::PathBuf;
use std::sync::RwLock;

use crate::compilation::{compile_cmd, odrefresh, CompilerOutput};
use crate::compos_key_service::CompOsKeyService;
use crate::fsverity;
use authfs_aidl_interface::aidl::com::android::virt::fs::IAuthFsService::IAuthFsService;
use compos_aidl_interface::aidl::com::android::compos::{
    CompOsKeyData::CompOsKeyData,
    CompilationResult::CompilationResult,
    FdAnnotation::FdAnnotation,
    ICompOsService::{BnCompOsService, ICompOsService},
};
use compos_aidl_interface::binder::{
    BinderFeatures, ExceptionCode, Interface, Result as BinderResult, Strong,
};

const AUTHFS_SERVICE_NAME: &str = "authfs_service";
const DEX2OAT_PATH: &str = "/apex/com.android.art/bin/dex2oat64";
const ODREFRESH_PATH: &str = "/apex/com.android.art/bin/odrefresh";

/// Constructs a binder object that implements ICompOsService.
pub fn new_binder() -> Result<Strong<dyn ICompOsService>> {
    let service = CompOsService {
        dex2oat_path: PathBuf::from(DEX2OAT_PATH),
        odrefresh_path: PathBuf::from(ODREFRESH_PATH),
        key_service: CompOsKeyService::new()?,
        key_blob: RwLock::new(Vec::new()),
    };
    Ok(BnCompOsService::new_binder(service, BinderFeatures::default()))
}

struct CompOsService {
    dex2oat_path: PathBuf,
    odrefresh_path: PathBuf,
    key_service: CompOsKeyService,
    key_blob: RwLock<Vec<u8>>,
}

impl CompOsService {
    fn generate_raw_fsverity_signature(
        &self,
        key_blob: &[u8],
        fsverity_digest: &fsverity::Sha256Digest,
    ) -> Vec<u8> {
        let formatted_digest = fsverity::to_formatted_digest(fsverity_digest);
        self.key_service.sign(key_blob, &formatted_digest[..]).unwrap_or_else(|e| {
            warn!("Failed to sign the fsverity digest, returning empty signature.  Error: {}", e);
            Vec::new()
        })
    }
}

impl Interface for CompOsService {}

impl ICompOsService for CompOsService {
    fn initializeSigningKey(&self, key_blob: &[u8]) -> BinderResult<()> {
        let mut w = self.key_blob.write().unwrap();
        if w.is_empty() {
            *w = Vec::from(key_blob);
            Ok(())
        } else {
            Err(new_binder_exception(ExceptionCode::ILLEGAL_STATE, "Cannot re-initialize the key"))
        }
    }

    fn initializeClasspaths(
        &self,
        boot_classpath: &str,
        dex2oat_boot_classpath: &str,
        system_server_classpath: &str,
    ) -> BinderResult<()> {
        // TODO(198211396): Implement correctly.
        env::set_var("BOOTCLASSPATH", boot_classpath);
        env::set_var("DEX2OATBOOTCLASSPATH", dex2oat_boot_classpath);
        env::set_var("SYSTEMSERVERCLASSPATH", system_server_classpath);
        Ok(())
    }

    fn odrefresh(
        &self,
        system_dir_fd: i32,
        output_dir_fd: i32,
        zygote_arch: &str,
    ) -> BinderResult<CompilationResult> {
        if system_dir_fd < 0 || output_dir_fd < 0 {
            return Err(new_binder_exception(
                ExceptionCode::ILLEGAL_ARGUMENT,
                "The remote FDs are expected to be non-negative",
            ));
        }
        if zygote_arch != "zygote64" && zygote_arch != "zygote64_32" {
            return Err(new_binder_exception(
                ExceptionCode::ILLEGAL_ARGUMENT,
                "Invalid zygote arch",
            ));
        }

        let authfs_service = get_authfs_service()?;
        let output = odrefresh(
            &self.odrefresh_path,
            system_dir_fd,
            output_dir_fd,
            zygote_arch,
            authfs_service,
        )
        .map_err(|e| {
            warn!("odrefresh failed: {}", e);
            new_binder_exception(
                ExceptionCode::SERVICE_SPECIFIC,
                format!("odrefresh failed: {}", e),
            )
        })?;
        match output {
            CompilerOutput::ExitCode(exit_code) => {
                Ok(CompilationResult { exitCode: exit_code, ..Default::default() })
            }
            _ => Err(new_binder_exception(ExceptionCode::SERVICE_SPECIFIC, "odrefresh failed")),
        }
    }

    fn compile_cmd(
        &self,
        args: &[String],
        fd_annotation: &FdAnnotation,
    ) -> BinderResult<CompilationResult> {
        let authfs_service = get_authfs_service()?;
        let output =
            compile_cmd(&self.dex2oat_path, args, authfs_service, fd_annotation).map_err(|e| {
                new_binder_exception(
                    ExceptionCode::SERVICE_SPECIFIC,
                    format!("Compilation failed: {}", e),
                )
            })?;
        match output {
            CompilerOutput::Digests { oat, vdex, image } => {
                let key = &*self.key_blob.read().unwrap();
                if key.is_empty() {
                    Err(new_binder_exception(
                        ExceptionCode::ILLEGAL_STATE,
                        "Key is not initialized",
                    ))
                } else {
                    let oat_signature = self.generate_raw_fsverity_signature(key, &oat);
                    let vdex_signature = self.generate_raw_fsverity_signature(key, &vdex);
                    let image_signature = self.generate_raw_fsverity_signature(key, &image);
                    Ok(CompilationResult {
                        exitCode: 0,
                        oatSignature: oat_signature,
                        vdexSignature: vdex_signature,
                        imageSignature: image_signature,
                    })
                }
            }
            CompilerOutput::ExitCode(exit_code) => {
                Ok(CompilationResult { exitCode: exit_code, ..Default::default() })
            }
        }
    }

    fn compile(&self, _marshaled: &[u8], _fd_annotation: &FdAnnotation) -> BinderResult<i8> {
        Err(new_binder_exception(ExceptionCode::UNSUPPORTED_OPERATION, "Not yet implemented"))
    }

    fn generateSigningKey(&self) -> BinderResult<CompOsKeyData> {
        self.key_service
            .generate()
            .map_err(|e| new_binder_exception(ExceptionCode::ILLEGAL_STATE, e.to_string()))
    }

    fn verifySigningKey(&self, key_blob: &[u8], public_key: &[u8]) -> BinderResult<bool> {
        Ok(if let Err(e) = self.key_service.verify(key_blob, public_key) {
            warn!("Signing key verification failed: {}", e.to_string());
            false
        } else {
            true
        })
    }

    fn sign(&self, data: &[u8]) -> BinderResult<Vec<u8>> {
        let key = &*self.key_blob.read().unwrap();
        if key.is_empty() {
            Err(new_binder_exception(ExceptionCode::ILLEGAL_STATE, "Key is not initialized"))
        } else {
            self.key_service
                .sign(key, data)
                .map_err(|e| new_binder_exception(ExceptionCode::ILLEGAL_STATE, e.to_string()))
        }
    }
}

fn get_authfs_service() -> BinderResult<Strong<dyn IAuthFsService>> {
    Ok(authfs_aidl_interface::binder::get_interface(AUTHFS_SERVICE_NAME)?)
}

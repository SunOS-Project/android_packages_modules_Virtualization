// Copyright 2021, The Android Open Source Project
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

//! Android VM control tool.

mod create_partition;
mod run;
mod sync;

use android_system_virtualizationservice::aidl::android::system::virtualizationservice::IVirtualizationService::IVirtualizationService;
use android_system_virtualizationservice::binder::{wait_for_interface, ProcessState, Strong};
use anyhow::{Context, Error};
use create_partition::command_create_partition;
use run::{command_run, command_run_app};
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;

const VIRTUALIZATION_SERVICE_BINDER_SERVICE_IDENTIFIER: &str =
    "android.system.virtualizationservice";

#[derive(StructOpt)]
#[structopt(no_version, global_settings = &[AppSettings::DisableVersion])]
enum Opt {
    /// Run a virtual machine with a config in APK
    RunApp {
        /// Path to VM Payload APK
        #[structopt(parse(from_os_str))]
        apk: PathBuf,

        /// Path to idsig of the APK
        #[structopt(parse(from_os_str))]
        idsig: PathBuf,

        /// Path to the instance image. Created if not exists.
        #[structopt(parse(from_os_str))]
        instance: PathBuf,

        /// Path to VM config JSON within APK (e.g. assets/vm_config.json)
        config_path: String,

        /// Detach VM from the terminal and run in the background
        #[structopt(short, long)]
        daemonize: bool,

        /// Path to file for VM log output.
        #[structopt(short, long)]
        log: Option<PathBuf>,

        /// Whether to run VM in debug mode.
        #[structopt(short, long)]
        debug: bool,
    },
    /// Run a virtual machine
    Run {
        /// Path to VM config JSON
        #[structopt(parse(from_os_str))]
        config: PathBuf,

        /// Detach VM from the terminal and run in the background
        #[structopt(short, long)]
        daemonize: bool,

        /// Path to file for VM log output.
        #[structopt(short, long)]
        log: Option<PathBuf>,
    },
    /// Stop a virtual machine running in the background
    Stop {
        /// CID of the virtual machine
        cid: u32,
    },
    /// List running virtual machines
    List,
    /// Create a new empty partition to be used as a writable partition for a VM
    CreatePartition {
        /// Path at which to create the image file
        #[structopt(parse(from_os_str))]
        path: PathBuf,

        /// The desired size of the partition, in bytes.
        size: u64,
    },
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let opt = Opt::from_args();

    // We need to start the thread pool for Binder to work properly, especially link_to_death.
    ProcessState::start_thread_pool();

    let service = wait_for_interface(VIRTUALIZATION_SERVICE_BINDER_SERVICE_IDENTIFIER)
        .context("Failed to find VirtualizationService")?;

    match opt {
        Opt::RunApp { apk, idsig, instance, config_path, daemonize, log, debug } => {
            command_run_app(
                service,
                &apk,
                &idsig,
                &instance,
                &config_path,
                daemonize,
                log.as_deref(),
                debug,
            )
        }
        Opt::Run { config, daemonize, log } => {
            command_run(service, &config, daemonize, log.as_deref())
        }
        Opt::Stop { cid } => command_stop(service, cid),
        Opt::List => command_list(service),
        Opt::CreatePartition { path, size } => command_create_partition(service, &path, size),
    }
}

/// Retrieve reference to a previously daemonized VM and stop it.
fn command_stop(service: Strong<dyn IVirtualizationService>, cid: u32) -> Result<(), Error> {
    service
        .debugDropVmRef(cid as i32)
        .context("Failed to get VM from VirtualizationService")?
        .context("CID does not correspond to a running background VM")?;
    Ok(())
}

/// List the VMs currently running.
fn command_list(service: Strong<dyn IVirtualizationService>) -> Result<(), Error> {
    let vms = service.debugListVms().context("Failed to get list of VMs")?;
    println!("Running VMs: {:#?}", vms);
    Ok(())
}

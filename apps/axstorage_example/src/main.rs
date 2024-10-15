//! An example of how to handle storage disks using the Edge Storage API.

use std::{
    ffi::CString,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::ExitCode,
    sync::{
        atomic::{AtomicU32, Ordering},
        Mutex,
    },
};

use axstorage::flex::{list, StatusEventId, Storage, StorageId, StorageType, SubscriptionId};
use glib::{ControlFlow, Error};
use log::{error, info, warn};

static DISKS_LIST: Mutex<Vec<DiskItem>> = Mutex::new(Vec::new());

#[derive(Debug)]
struct DiskItem {
    storage: Option<Storage>,
    storage_type: Option<StorageType>,
    storage_id: StorageId,
    storage_path: Option<CString>,
    subscription_id: SubscriptionId,
    setup: bool,
    writable: bool,
    available: bool,
    full: bool,
    exiting: bool,
}

fn write_data(data: &str) -> ControlFlow {
    static COUNTER: AtomicU32 = AtomicU32::new(1);

    for item in DISKS_LIST.lock().unwrap().iter() {
        if item.available && item.writable && !item.full && item.setup {
            let filename =
                PathBuf::from(item.storage_path.as_ref().unwrap().to_str().unwrap()).join(data);
            let file = match OpenOptions::new().append(true).create(true).open(&filename) {
                Ok(f) => f,
                Err(e) => {
                    warn!("Failed to open {filename:?}. Error: {e:?}");
                    return ControlFlow::Break;
                }
            };
            let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
            if let Err(e) = writeln!(&file, "counter: {counter}") {
                warn!("Failed to write to {filename:?} because {e:?}");
                return ControlFlow::Break;
            }
            drop(file);
            info!("Writing to {filename:?}");
        }
    }
    ControlFlow::Continue
}

fn find_disk_item<'a>(
    disks_list: &'a mut Vec<DiskItem>,
    storage_id: &StorageId,
) -> Option<&'a mut DiskItem> {
    disks_list
        .iter_mut()
        .find(|item| item.storage_id == *storage_id)
}

fn release_disk_cb(storage_id: &StorageId, result: Option<Error>) {
    info!("Release of {storage_id}");
    if let Some(e) = result {
        warn!("Error while releasing {storage_id}: {e:?}")
    }
}

fn free_disk_item() {
    for item in DISKS_LIST.lock().unwrap().drain(..) {
        if item.setup {
            match item.storage.unwrap().release_async({
                let storage_id = item.storage_id.clone();
                move |r| release_disk_cb(&storage_id, r)
            }) {
                Ok(()) => info!("Release of {} was successful", item.storage_id),
                Err(e) => warn!("Failed to release {}. Error: {e:?}", item.storage_id),
            }
        }

        match item.subscription_id.unsubscribe() {
            Ok(()) => info!(
                "Unsubscribed events of {:?}",
                item.storage_path.as_ref().unwrap()
            ),
            Err(e) => warn!(
                "Failed to unsubscribe event of {:?}. Error: {e:?}",
                item.storage_path.as_ref().unwrap()
            ),
        }
    }
}

fn setup_disk_cb(storage: Result<Storage, Error>) {
    let mut storage = match storage {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to setup disk. Error: {e:?}");
            return;
        }
    };

    let storage_id = match storage.get_storage_id() {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get storage_id. Error: {e:?}");
            return;
        }
    };

    let path = match storage.get_path() {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get storage path. Error: {e:?}");
            return;
        }
    };

    let storage_type = match storage.get_type() {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get storage type. Error: {e:?}");
            return;
        }
    };

    let mut disks_list = DISKS_LIST.lock().unwrap();
    let disk = find_disk_item(&mut disks_list, &storage_id).unwrap();
    disk.storage = Some(storage);
    disk.storage_type = Some(storage_type);
    disk.storage_path = Some(path.clone());
    disk.setup = true;

    info!("Disk: {storage_id} has been setup in {path:?}");
}

fn subscribe_cb(storage_id: &mut StorageId, error: Option<Error>) {
    if let Some(e) = error {
        warn!("Failed to subscribe to {storage_id}. Error: {e:?}");
        return;
    }

    info!("Subscribe for the events of {storage_id}");
    let mut disks_list = DISKS_LIST.lock().unwrap();
    let disk = find_disk_item(&mut disks_list, &storage_id).unwrap();

    let exiting = match storage_id.get_status(StatusEventId::Exiting) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get EXITING event for {storage_id}. Error: {e:?}");
            return;
        }
    };

    let available = match storage_id.get_status(StatusEventId::Available) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get AVAILABLE event for {storage_id}. Error: {e:?}");
            return;
        }
    };

    let writable = match storage_id.get_status(StatusEventId::Writable) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get WRITABLE event for {storage_id}. Error: {e:?}");
            return;
        }
    };

    let full = match storage_id.get_status(StatusEventId::Full) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get FULL event for {storage_id}. Error: {e:?}");
            return;
        }
    };

    disk.writable = writable;
    disk.available = available;
    disk.exiting = exiting;
    disk.full = full;

    info!(
        "Status of events for {storage_id}: {}writable, {}available, {}exiting, {}full",
        if writable { "" } else { "not " },
        if available { "" } else { "not " },
        if exiting { "" } else { "not " },
        if full { "" } else { "not " },
    );

    if disk.exiting && disk.setup {
        match disk.storage.as_mut().unwrap().release_async({
            let storage_id = storage_id.clone();
            move |r| release_disk_cb(&storage_id, r)
        }) {
            Ok(()) => {
                info!("Release of {storage_id} was successful");
                disk.setup = false;
            }
            Err(e) => warn!("Failed to release {storage_id}. Error: {e:?}"),
        }
    } else if disk.writable && !disk.full && !disk.setup {
        info!("Setup {storage_id}");
        match storage_id.setup_async(setup_disk_cb) {
            Ok(()) => info!("Setup of {storage_id} was successful"),
            Err(e) => warn!("Failed to setup {storage_id}, reason: {e:?}"),
        }
    }
}

fn new_disk_item(mut storage_id: StorageId) -> Option<DiskItem> {
    let subscription_id = match storage_id.subscribe(subscribe_cb) {
        Ok(t) => t,
        Err(e) => {
            error!("Failed to subscribe to events of {storage_id}. Error: {e:?}");
            return None;
        }
    };

    let item = DiskItem {
        storage: None,
        storage_type: None,
        storage_id,
        storage_path: None,
        subscription_id,
        setup: false,
        writable: false,
        available: false,
        full: false,
        exiting: false,
    };
    Some(item)
}

fn main() -> ExitCode {
    acap_logging::init_logger();

    let disks = match list() {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to list storage devices. Error: {e:?}");
            info!("Finish AXStorage application");
            return ExitCode::FAILURE;
        }
    };

    let main_loop = glib::MainLoop::new(None, false);

    for disk_name in disks.into_iter() {
        let item = match new_disk_item(disk_name.clone()) {
            Some(t) => t,
            None => {
                warn!("{disk_name} is skipped");
                continue;
            }
        };
        println!("push...");
        DISKS_LIST.lock().unwrap().push(item);
        println!("pushed.");
    }

    glib::timeout_add_seconds(10, || write_data("file1"));
    glib::timeout_add_seconds(10, || write_data("file2"));

    main_loop.run();

    free_disk_item();

    info!("Finish AXStorage application");
    ExitCode::SUCCESS
}

#[cfg(not(target_arch = "x86_64"))]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_test() {}
}

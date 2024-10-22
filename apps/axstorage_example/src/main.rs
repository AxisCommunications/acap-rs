//! An example of how to handle storage disks using the Edge Storage API.

use std::{
    cell::{Cell, RefCell},
    ffi::CString,
    fs::OpenOptions,
    io::Write,
    path::PathBuf,
    process::ExitCode,
};

use axstorage::flex::{StatusEventId, Storage, StorageId, Type};
use glib::{ControlFlow, Error};
use libc::{SIGINT, SIGTERM};
use log::{error, info, warn};

thread_local! {
    static DISKS_LIST: RefCell<Vec<DiskItem>> = const { RefCell::new(Vec::new()) };
}

#[derive(Debug)]
struct DiskItem {
    storage: Option<Storage>,
    storage_type: Option<Type>,
    storage_id: StorageId,
    storage_path: Option<CString>,
    subscription_id: u32,
    setup: bool,
    writable: bool,
    available: bool,
    full: bool,
    exiting: bool,
}

fn write_data(data: &str) -> ControlFlow {
    thread_local! {static COUNTER: Cell<u32> = const { Cell::new(0) }}
    DISKS_LIST.with_borrow(|disks_list| {
        for item in disks_list.iter() {
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
                let mut counter = COUNTER.get();
                counter += 1;
                COUNTER.set(counter);
                if let Err(e) = writeln!(&file, "counter: {counter}") {
                    warn!("Failed to write to {filename:?} because {e:?}");
                    return ControlFlow::Break;
                }
                drop(file);
                info!("Writing to {filename:?}");
            }
        }
        ControlFlow::Continue
    })
}

fn find_disk_item<'a>(
    disks_list: &'a mut [DiskItem],
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
    let mut disks_list = DISKS_LIST.take();
    for item in disks_list.drain(..) {
        if item.setup {
            match axstorage::flex::release_async(&mut item.storage.unwrap(), {
                let storage_id = item.storage_id.clone();
                Some(move |r| release_disk_cb(&storage_id, r))
            }) {
                Ok(()) => info!("Release of {} was successful", item.storage_id),
                Err(e) => warn!("Failed to release {}. Error: {e:?}", item.storage_id),
            }
        }

        match axstorage::flex::unsubscribe(item.subscription_id) {
            Ok(()) => info!("Unsubscribed events of {:?}", item.storage_id),
            Err(e) => warn!(
                "Failed to unsubscribe event of {:?}. Error: {e:?}",
                item.storage_id
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

    let storage_id = match axstorage::flex::get_storage_id(&mut storage) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get storage_id. Error: {e:?}");
            return;
        }
    };

    let path = match axstorage::flex::get_path(&mut storage) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get storage path. Error: {e:?}");
            return;
        }
    };

    let storage_type = match axstorage::flex::get_type(&mut storage) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to get storage type. Error: {e:?}");
            return;
        }
    };

    DISKS_LIST.with_borrow_mut(|disks_list| {
        let disk = find_disk_item(disks_list, &storage_id).unwrap();
        disk.storage = Some(storage);
        disk.storage_type = Some(storage_type);
        disk.storage_path = Some(path.clone());
        disk.setup = true;
    });

    info!("Disk: {storage_id} has been setup in {path:?}");
}

fn subscribe_cb(storage_id: &mut StorageId, error: Option<Error>) {
    if let Some(e) = error {
        warn!("Failed to subscribe to {storage_id}. Error: {e:?}");
        return;
    }

    info!("Subscribe for the events of {storage_id}");
    DISKS_LIST.with_borrow_mut(|disks_list| {
        let disk = find_disk_item(disks_list, storage_id).unwrap();

        let exiting = match axstorage::flex::get_status(storage_id, StatusEventId::Exiting) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to get EXITING event for {storage_id}. Error: {e:?}");
                return;
            }
        };

        let available = match axstorage::flex::get_status(storage_id, StatusEventId::Available) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to get AVAILABLE event for {storage_id}. Error: {e:?}");
                return;
            }
        };

        let writable = match axstorage::flex::get_status(storage_id, StatusEventId::Writable) {
            Ok(t) => t,
            Err(e) => {
                warn!("Failed to get WRITABLE event for {storage_id}. Error: {e:?}");
                return;
            }
        };

        let full = match axstorage::flex::get_status(storage_id, StatusEventId::Full) {
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
            match axstorage::flex::release_async(disk.storage.as_mut().unwrap(), {
                let storage_id = storage_id.clone();
                Some(move |r| release_disk_cb(&storage_id, r))
            }) {
                Ok(()) => {
                    info!("Release of {storage_id} was successful");
                    disk.setup = false;
                }
                Err(e) => warn!("Failed to release {storage_id}. Error: {e:?}"),
            }
        } else if disk.writable && !disk.full && !disk.setup {
            info!("Setup {storage_id}");
            match axstorage::flex::setup_async(storage_id, Some(setup_disk_cb)) {
                Ok(()) => info!("Setup of {storage_id} was successful"),
                Err(e) => warn!("Failed to setup {storage_id}, reason: {e:?}"),
            }
        }
    })
}

fn new_disk_item(mut storage_id: StorageId) -> Option<DiskItem> {
    let subscription_id = match axstorage::flex::subscribe(&mut storage_id, subscribe_cb) {
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

    let disks = match axstorage::flex::list() {
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
        DISKS_LIST.with_borrow_mut(|disks_list| disks_list.push(item));
    }

    glib::timeout_add_seconds(10, || write_data("file1"));
    glib::timeout_add_seconds(10, || write_data("file2"));
    glib::unix_signal_add(SIGTERM, {
        let main_loop = main_loop.clone();
        move || {
            main_loop.quit();
            ControlFlow::Continue
        }
    });
    glib::unix_signal_add(SIGINT, {
        let main_loop = main_loop.clone();
        move || {
            main_loop.quit();
            ControlFlow::Continue
        }
    });

    main_loop.run();

    free_disk_item();

    info!("Finish AXStorage application");
    ExitCode::SUCCESS
}

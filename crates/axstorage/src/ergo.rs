use crate::flex;
use crate::flex::Type;
use glib::GStringPtr;

pub struct StorageId(GStringPtr);

impl StorageId {
    pub fn list() -> Result<Vec<Self>, glib::Error> {
        flex::list().map(|l| l.into_iter().map(Self).collect())
    }

    pub async fn setup(&mut self) -> Result<Storage, glib::Error> {
        todo!()
    }

    pub fn subscribe(&mut self) -> Result<Subscription, glib::Error> {
        todo!()
    }

    pub fn is_available(&mut self) -> Result<bool, glib::Error> {
        flex::get_status(&mut self.0, flex::StatusEventId::Available)
    }
    pub fn is_exiting(&mut self) -> Result<bool, glib::Error> {
        flex::get_status(&mut self.0, flex::StatusEventId::Exiting)
    }
    pub fn is_full(&mut self) -> Result<bool, glib::Error> {
        flex::get_status(&mut self.0, flex::StatusEventId::Full)
    }

    pub fn is_writable(&mut self) -> Result<bool, glib::Error> {
        flex::get_status(&mut self.0, flex::StatusEventId::Writable)
    }
}

pub struct Storage(flex::Storage);

impl Drop for Storage {
    fn drop(&mut self) {
        // Leak callback and panic if release has not been successfully called
    }
}
impl Storage {
    pub async fn release(self) -> Result<Option<glib::Error>, Storage> {
        todo!()
    }

    pub fn get_path(&mut self) -> Result<std::path::PathBuf, glib::Error> {
        flex::get_path(&mut self.0).map(|p| std::path::PathBuf::from(p.into_string().unwrap()))
    }
    pub fn get_storage_id(&mut self) -> Result<StorageId, glib::Error> {
        flex::get_storage_id(&mut self.0).map(StorageId)
    }
    pub fn get_type(&mut self) -> Result<Type, glib::Error> {
        flex::get_type(&mut self.0)
    }
}

pub struct Subscription {}

impl Drop for Subscription {
    fn drop(&mut self) {
        // Leak callback and panic if unsubscribe has not been successfully called
        todo!()
    }
}

// TODO: implement async stream for Subscription

impl Subscription {
    pub async fn unsubscribe(&self) -> Result<Option<glib::Error>, Storage> {
        todo!()
    }
}

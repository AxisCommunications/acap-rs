use larod::{Error, Session};

fn main() -> Result<(), Error> {
    let session = Session::new();
    let devices = match session.devices() {
        Ok(d) => d,
        Err(Error::LarodError(e)) => {
            if let Ok(msg) = e.msg() {
                eprintln!("Error while listing available devices! {}", msg);
            } else {
                eprintln!("Error while listing available devices. Error returned ")
            }
            return Err(Error::LarodError(e));
        }
        Err(e) => {
            eprintln!("Unknown error while listing devices: {:?}", e);
            return Err(e);
        }
    };
    println!("Devices:");
    for d in devices {
        println!(
            "{} ({})",
            d.name().expect("Couldn't get device name"),
            d.instance().expect("Couldn't get device instance")
        );
    }
    Ok(())
}

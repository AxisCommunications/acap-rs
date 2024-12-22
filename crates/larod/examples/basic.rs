use larod::{Error, ImageFormat, LarodModel, PreProcBackend, Preprocessor, Session};

fn main() -> Result<(), Error> {
    env_logger::init();
    let session = Session::new();
    let devices = match session.devices() {
        Ok(d) => d,
        Err(Error::LarodError(e)) => {
            if let Ok(msg) = e.msg() {
                log::error!("Error while listing available devices! {}", msg);
            } else {
                log::error!("Error while listing available devices. Error returned ")
            }
            return Err(Error::LarodError(e));
        }
        Err(e) => {
            log::error!("Unknown error while listing devices: {:?}", e);
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
    let mut preprocessor = Preprocessor::builder()
        .input_format(ImageFormat::NV12)
        .input_size(1920, 1080)
        .output_size(1920, 1080)
        .backend(PreProcBackend::LibYUV)
        .load(&session)?;
    if let Err(Error::LarodError(e)) = preprocessor.create_model_inputs() {
        log::error!("Error creating preprocessor inputs: {:?}", e.msg());
    }
    if let Some(tensors) = preprocessor.input_tensors() {
        log::info!("input_tensor size: {}", tensors.len());
        for t in tensors.iter() {
            log::info!("first_tensor dims {:?}", t.dims());
        }
    }
    Ok(())
}

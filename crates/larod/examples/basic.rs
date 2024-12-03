use larod::{Error, ImageFormat, LarodModel, PreProcBackend, Preprocessor, Session};

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
    let mut preprocessor = match Preprocessor::builder()
        .input_format(ImageFormat::NV12)
        .input_size((1920, 1080))
        .output_size((1920, 1080))
        .backend(PreProcBackend::LibYUV)
        .load(&session)
    {
        Ok(p) => p,
        Err(Error::LarodError(e)) => {
            eprintln!("Error building preprocessor: {:?}", e.msg());
            panic!()
        }
        Err(e) => {
            eprintln!("Unexpected error while building preprocessor: {:?}", e);
            panic!()
        }
    };
    if let Err(Error::LarodError(e)) = preprocessor.create_model_inputs() {
        eprintln!("Error creating preprocessor inputs: {:?}", e.msg());
    }
    println!("Number of model inputs: {}", preprocessor.num_inputs());
    Ok(())
}

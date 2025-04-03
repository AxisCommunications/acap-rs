use anyhow::Context;
use larod::{ImageFormat, LarodModel, PreProcBackend, Preprocessor, Session};
use std::{fs, iter::Iterator, path::Path};

fn get_file(name: &str) -> anyhow::Result<std::fs::File> {
    let path = Path::new(name);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    fs::remove_file(path)?;
    Ok(file)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let session = Session::new();
    let devices = session
        .devices()
        .context("could not list available devices")?;
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
    preprocessor
        .create_model_inputs()
        .context("Error while creating model inputs")?;
    if let Some(tensors) = preprocessor.input_tensors_mut() {
        log::info!("input_tensor size: {}", tensors.len());
        for t in tensors.iter_mut() {
            log::info!("first_tensor layout {:?}", t.layout());
            log::info!("first_tensor dims {:?}", t.dims());
            log::info!("first_tensor pitches {:?}", t.pitches());
            let pitches = t.pitches()?;
            let file = get_file("/tmp/acap-rs-larod-preproc-1")?;
            file.set_len(pitches[0] as u64)?;
            t.set_buffer(file)?;
        }
    }
    let Some(input_tensor) = preprocessor
        .input_tensors_mut()
        .and_then(|tc| tc.first_mut())
    else {
        return Err(anyhow::anyhow!("preprocessor has no input tensors"));
    };

    Ok(())
}

//! # Simple example
//! - Download model
//! ```bash
//! curl -o model.zi https://acap-artifacts.s3.eu-north-1.amazonaws.com/models/models.aarch64.artpec8.zip
//! unzip -q model.zip && rm -f model.zip
//! ```
//! - Prepare input data
//! ```bash
//! curl -o car.jpg https://upload.wikimedia.org/wikipedia/commons/f/f4/Opel_Olympia_Rekord_P1_Kombi_2012-09-01_14-29-57.JPG
//! python3 -m pip install numpy pillow
//! echo "import numpy as np
//! from PIL import Image
//! img = Image.open("car.jpg")
//! small_img = img.resize((270, 480))
//! small_np_img = np.asarray(small_img)
//! small_np_img.tofile("car.bin")" >> data_prep.py
//! python3 data_prep.py
//! ```
//! - Copy model/converted_model.tflite and car.bin to test camera
//! - Build and execute the code on the camera.
//!
//!   If you've already set the `AXIS_DEVICE_IP` environment variable, it should be as simple as `cargo run -p larod --example simple-file`
use anyhow::Context;
use larod::{
    Artpec8DLPU, InferenceModel, LarodModel, Session, TFLite, TensorDataType, TensorLayout,
};
use std::fmt::Write;
use std::io::Seek;
use std::{iter::Iterator, path::Path};

fn setup_inference(session: &Session) -> anyhow::Result<InferenceModel> {
    log::info!("Setting up inference model");
    let mut model = InferenceModel::new::<TFLite, Artpec8DLPU, _>(
        session,
        "converted_model.tflite",
        larod::LarodAccess::LAROD_ACCESS_PRIVATE,
        "Example Model",
        None,
    )?;
    model.create_model_inputs()?;
    model.create_model_outputs()?;
    log::info!("Inference model input tensors");
    log::info!("Number of model inputs: {}", model.num_inputs());

    if let Some(tensors) = model.input_tensors_mut() {
        for t in tensors.iter_mut() {
            t.set_layout(TensorLayout::LAROD_TENSOR_LAYOUT_NHWC)?;
            t.set_data_type(TensorDataType::LAROD_TENSOR_DATA_TYPE_UINT8)?;
            let pitches = t.pitches()?;
            let file = get_file("car.bin")?;
            t.set_buffer(file)?;
            t.set_fd_props(larod::FDAccessFlag::TYPE_DISK)?;
            log::info!("Tensor: {:?}", t.name());
            log::info!("first_tensor layout {:?}", t.layout());
            log::info!("first_tensor dims {:?}", t.dims());
            log::info!("first_tensor pitches {:?}", t.pitches());
            log::info!("first_tensor byte_size {:?}", t.byte_size());

            log::info!(
                "input tensor offset: {}",
                t.fd_offset().expect("unable to get tensor offset")
            );
        }
    }

    log::info!("Inference model output tensors");
    log::info!("Number of model outputs: {}", model.num_outputs());
    if let Some(tensors) = model.output_tensors_mut() {
        for t in tensors.iter_mut() {
            t.set_data_type(TensorDataType::LAROD_TENSOR_DATA_TYPE_UINT8)?;
            log::info!("Tensor: {:?}", t.name());
            log::info!("first_tensor layout {:?}", t.layout());
            log::info!("first_tensor dims {:?}", t.dims());
            log::info!("first_tensor pitches {:?}", t.pitches());
            let pitches = t.pitches()?;
            let file = get_file("output_tensor.bin")?;

            file.set_len(pitches[0] as u64)?;
            log::debug!("Setting output tensor file descriptor to {:?}", file);
            t.set_buffer(file)?;
            t.set_fd_props(larod::FDAccessFlag::TYPE_DISK)?;
        }
    }
    Ok(model)
}

fn get_file(name: &str) -> anyhow::Result<std::fs::File> {
    let path = Path::new(name);
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)?;
    Ok(file)
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let session = Session::new();
    let devices = session
        .devices()
        .context("Couldn't list available devices")?;
    log::info!("Devices:");
    for d in devices {
        log::info!(
            "{} ({})",
            d.name().expect("Couldn't get device name"),
            d.instance().expect("Couldn't get device instance")
        );
    }

    let mut inference = setup_inference(&session)?;
    let inference_job_request = inference.create_job()?;

    if let Some(ot) = inference.output_tensors_mut() {
        for b in ot.iter_mut() {
            if let Some(f) = b.buffer_mut() {
                f.rewind().expect("failed to rewind outbut tensor buffer")
            }
        }
    }

    inference_job_request.run()?;
    log::info!("Ran inference job");
    if let Some(ot) = inference.output_tensors_mut() {
        for b in ot.iter_mut() {
            if let Some(f) = b.buffer_mut() {
                f.rewind().expect("failed to rewind outbut tensor buffer")
            }
        }
    }
    if let Some(inf_slice) = inference
        .output_tensors()
        .and_then(|ot| ot.first())
        .and_then(|ft| ft.as_slice())
    {
        log::info!("Results\n{:?}", inf_slice);
    }
    Ok(())
}

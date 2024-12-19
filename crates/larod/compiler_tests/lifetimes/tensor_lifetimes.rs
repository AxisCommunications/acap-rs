use larod::{Error, ImageFormat, LarodModel, PreProcBackend, Preprocessor, Session};
fn main() {
    env_logger::builder().is_test(true).try_init();
    let session = Session::new();
    let mut preprocessor = match Preprocessor::builder()
        .input_format(ImageFormat::NV12)
        .input_size(1920, 1080)
        .output_size(1920, 1080)
        .backend(PreProcBackend::LibYUV)
        .load(&session)
    {
        Ok(p) => p,
        Err(Error::LarodError(e)) => {
            log::error!("Error building preprocessor: {:?}", e.msg());
            panic!()
        }
        Err(e) => {
            log::error!("Unexpected error while building preprocessor: {:?}", e);
            panic!()
        }
    };
    if let Err(Error::LarodError(e)) = preprocessor.create_model_inputs() {
        log::error!("Error creating preprocessor inputs: {:?}", e.msg());
    }
    let tensors = preprocessor.input_tensors();
    drop(preprocessor);
    if let Some(tensors) = tensors {
        log::info!("input_tensor size: {}", tensors.len());
        for t in tensors.iter() {
            log::info!("first_tensor dims {:?}", t.dims());
        }
    }
}


/// Converts an image from any `image`-supported format to WEBP.
/// # Parameters
/// * `file_path`: Path to the image to convert.
/// # Returns
/// `Result<PathBuf, io::Error>`: Path of the WEBP-image as a `PathBuf` when
///  succesful, otherwise an `Error`.
pub fn image_to_webp<P: AsRef<Path>>(
    file_path: P,
) -> Result<PathBuf, Box<dyn Error + Send + Sync + 'static>> {
    // Open path as DynamicImage
    let file_path = file_path.as_ref();
    let image = ImageReader::open(file_path)?
        .with_guessed_format()?
        .decode()?;

    // Make webp::Encoder from DynamicImage.
    let encoder: Encoder = Encoder::from_image(&image)?;

    // Encode image into WebPMemory.
    let encoded_webp: WebPMemory = encoder.encode(65f32);

    // Since we opened the `file_path` successfully, we assume the path has a
    // parent and file component.
    let parent_directory = file_path.parent().unwrap();
    let filename_original_image = file_path.file_name().unwrap();

    // Put webp-image in a separate webp-folder in the location of the original image.
    let mut path = parent_directory.to_owned();
    path.push("webp");
    std::fs::create_dir_all(&path)?;

    // Get filename of target.
    path.push(filename_original_image);
    path.set_extension("webp");

    // Save webp image to file
    std::fs::write(&path, &*encoded_webp)?;

    Ok(path)
}



fn convert_webp_to_png(input_path: &str, output_path: &str) -> Result<(), image::ImageError> {
    // Load the WebP image
    let img = ImageReader::open(input_path)?
        .with_guessed_format()?
        .decode()?;

    // Save the image as PNG
    img.save_with_format(output_path, ImageFormat::Png)?;

    Ok(())
}

use image::GenericImage;

pub fn concat_jpeg_tile(
    width: u32,
    height: u32,
    images: &Vec<bytes::Bytes>,
) -> anyhow::Result<bytes::Bytes> {
    let decoded_images = images
        .iter()
        .map(|img| {
            image::io::Reader::with_format(std::io::Cursor::new(img), image::ImageFormat::Jpeg)
                .decode()
        })
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let tile_size = 256;
    let horizontal_count = (width + tile_size - 1) / tile_size;
    let mut output_image = image::RgbImage::new(width, height);
    for (i, decoded_image) in decoded_images.iter().enumerate() {
        let x = (i as u32 % horizontal_count) * tile_size;
        let y = (i as u32 / horizontal_count) * tile_size;
        output_image.copy_from(&decoded_image.to_rgb8(), x, y)?;
    }

    let mut output_bytes_vector: Vec<u8> = Vec::new();
    output_image.write_to(
        &mut std::io::Cursor::new(&mut output_bytes_vector),
        image::ImageFormat::Jpeg,
    )?;
    let output_bytes = bytes::Bytes::copy_from_slice(&output_bytes_vector);
    Ok(output_bytes)
}

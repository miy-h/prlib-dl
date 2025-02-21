use image::GenericImage;

pub fn concat_jpeg_tile(
    width: u32,
    height: u32,
    images: &[bytes::Bytes],
) -> anyhow::Result<bytes::Bytes> {
    let decoded_images = images
        .iter()
        .map(|img| {
            image::ImageReader::with_format(std::io::Cursor::new(img), image::ImageFormat::Jpeg)
                .decode()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let tile_size = 256;
    let horizontal_count = width.div_ceil(tile_size);
    let mut output_image = image::RgbImage::new(width, height);
    for (decoded_image, i) in decoded_images.iter().zip(0_u32..) {
        let x = (i % horizontal_count) * tile_size;
        let y = (i / horizontal_count) * tile_size;
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

use std::{fs::File, io::BufWriter, iter, path::Path};

use wgpu::{Device, Queue, Texture};

pub async fn save_png(tex: &Texture, size: [u32; 2], device: &Device, queue: &Queue, path: &str) {
    let output_buffer_size = (size[0] * size[1] * 4) as wgpu::BufferAddress;
    let output_buffer_desc = wgpu::BufferDescriptor {
        size: output_buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        label: Some("Ray DST"),
        mapped_at_creation: false,
    };

    let output_buffer = device.create_buffer(&output_buffer_desc);
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Render Encoder"),
    });
    let texture_extent = wgpu::Extent3d {
        width: size[0] as u32,
        height: size[1] as u32,
        depth_or_array_layers: 1,
    };
    encoder.copy_texture_to_buffer(
        tex.as_image_copy(),
        wgpu::ImageCopyBuffer {
            buffer: &output_buffer,
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(std::num::NonZeroU32::new(size[0] * 4).unwrap()),
                rows_per_image: None,
            },
        },
        texture_extent,
    );

    queue.submit(iter::once(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let buffer_future = buffer_slice.map_async(wgpu::MapMode::Read);
    device.poll(wgpu::Maintain::Wait);

    if let Ok(()) = buffer_future.await {
        let mut data = buffer_slice.get_mapped_range().to_vec();
        // BGRa TO RGBa
        data.chunks_mut(4).for_each(|chunk| chunk.swap(0, 2));

        let path = Path::new(path);
        let file = File::create(path).unwrap();
        let w = &mut BufWriter::new(file);
        let mut encoder = png::Encoder::new(w, size[0], size[1]);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();

        // let data = [255, 0, 0, 255, 0, 0, 0, 255]; // An array containing a RGBA sequence. First pixel is red and second pixel is black.
        writer.write_image_data(&data).unwrap(); // Save
    } else {
        panic!("Failed to copy texture to CPU!")
    }
}

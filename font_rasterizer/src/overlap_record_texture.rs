pub struct OverlapRecordBuffer {
    pub buffer: wgpu::Buffer,
}

impl OverlapRecordBuffer {
    pub fn new(device: &wgpu::Device, size: (u32, u32)) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Overlap Record Buffer"),
            size: (size.0 * size.1 * 4 * 3) as wgpu::BufferAddress, // Assuming 4 bytes per pixel
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { buffer }
    }

    pub fn clear(&self, queue: &wgpu::Queue) {
        let size = self.buffer.size();
        queue.write_buffer(&self.buffer, 0, &vec![0u8; size as usize]);
    }
}

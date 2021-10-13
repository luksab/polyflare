use async_trait::async_trait;
use wgpu::{Device, Queue, SurfaceConfiguration, TextureView};

#[async_trait]
pub trait Scene {
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>, device: &wgpu::Device, config: &SurfaceConfiguration);
    fn input(&mut self, event: &winit::event::WindowEvent) -> bool;
    fn update(&mut self, dt: std::time::Duration, queue: &Queue);
    fn render(
        &mut self,
        view: &TextureView,
        depth_view: Option<&TextureView>,
        device: &wgpu::Device,
        queue: &Queue,
    ) -> Result<(), wgpu::SurfaceError>;
}

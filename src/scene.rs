use async_trait::async_trait;
use wgpu::{Device, Queue, SurfaceConfiguration, SurfaceError, TextureView};

#[async_trait]
pub trait Scene {
    fn resize(
        &mut self,
        new_size: winit::dpi::PhysicalSize<u32>,
        scale_factor: f64,
        device: &Device,
        config: &SurfaceConfiguration,
        queue: &Queue,
    );
    fn input(&mut self, event: &winit::event::WindowEvent) -> bool;
    fn update(&mut self, dt: std::time::Duration, device: &Device, queue: &Queue);
    fn render(
        &mut self,
        view: &TextureView,
        depth_view: Option<&TextureView>,
        device: &Device,
        queue: &Queue,
    ) -> Result<(), SurfaceError>;
}

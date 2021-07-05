use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Instance {
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl Instance {
    pub async fn new(
        window: &winit::window::Window,
        width: u32,
        height: u32,
    ) -> anyhow::Result<(Instance, Swapchain)> {
        let instance = wgpu::Instance::new(wgpu::BackendBit::PRIMARY);
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
            })
            .await
            .expect("failed to acquire adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Rendering Device"),
                    features: wgpu::Features::default(),
                    limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let desc = wgpu::SwapChainDescriptor {
            format: adapter
                .get_swap_chain_preferred_format(&surface)
                .expect("no preferred format"),
            present_mode: wgpu::PresentMode::Fifo,
            width,
            height,
            usage: wgpu::TextureUsage::RENDER_ATTACHMENT,
        };
        let swap_chain = device.create_swap_chain(&surface, &desc);

        let instance = Self {
            device: Arc::new(device),
            queue: Arc::new(queue),
        };

        let swap_chain = Swapchain {
            surface,
            swap_chain,
            desc,
        };

        Ok((instance, swap_chain))
    }
}

pub struct Swapchain {
    pub surface: wgpu::Surface,
    pub swap_chain: wgpu::SwapChain,
    pub desc: wgpu::SwapChainDescriptor,
}

impl Swapchain {
    #[inline]
    pub fn format(&self) -> wgpu::TextureFormat {
        self.desc.format
    }

    #[inline]
    pub fn resize(&mut self, instance: &Instance, width: u32, height: u32) {
        self.desc.width = width;
        self.desc.height = height;
        self.swap_chain = instance.device.create_swap_chain(&self.surface, &self.desc);
    }

    #[inline]
    pub fn next_frame(&self) -> Result<wgpu::SwapChainFrame, wgpu::SwapChainError> {
        self.swap_chain.get_current_frame()
    }
}

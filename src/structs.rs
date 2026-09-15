use anyhow::*;
use thiserror::Error;
use winit::window::Window;

use vulkanalia::{
	loader::{
		LIBRARY, 
		LibloadingLoader}, 
		prelude::v1_0::*, 
	vk::{
			ExtDebugUtilsExtensionInstanceCommands, 
			KhrSurfaceExtensionInstanceCommands, KhrSwapchainExtensionDeviceCommands
	},
	window as vk_window 
};

use crate::functions::{
	VALIDATION_ENABLED, 
	create_instance, 
	create_logical_device, 
	create_pipeline,
	create_swapchain, 
	create_swapchain_image_views, 
	pick_physical_device
};

/// Our Vulkan app.
#[derive(Clone, Debug)]
pub struct App {
	pub entry: Entry,
	pub instance: Instance,
	pub data: AppData,
	pub device: Device
}

impl App {
	/// Creates our Vulkan app.
	pub unsafe fn create(window: &Window) -> Result<Self> {
		let loader = LibloadingLoader::new(LIBRARY)?;
		let entry = Entry::new(loader).map_err(|b| anyhow!("{}", b))?;
		let mut data = AppData::default();
		let instance = create_instance(window, &entry, &mut data)?;
		data.surface = vk_window::create_surface(&instance, &window, &window)?;
		pick_physical_device(&instance, &mut data)?;
		let device = create_logical_device(&entry, &instance, &mut data)?;
		create_swapchain(window, &instance, &device, &mut data)?;
		create_swapchain_image_views(&device, &mut data)?;
		create_pipeline(&device, &mut data)?;
		Ok(Self { entry, instance, data, device})
	}

	
	

	/// Renders a frame for our Vulkan app.
	pub unsafe fn render(&mut self, window: &Window) -> Result<()> {
		Ok(())
	}

	/// Destroys our Vulkan app.
	pub unsafe fn destroy(&mut self) {
		self.device.destroy_pipeline(self.data.pipeline, None);
		self.device.destroy_pipeline_layout(self.data.pipeline_layout, None);
		self.device.destroy_render_pass(self.data.render_pass, None);
		self.data.swapchain_image_views
					.iter()
					.for_each(|v| self.device
										.destroy_image_view(*v, None));
		self.device.destroy_swapchain_khr(self.data.swapchain, None);
		self.device.destroy_device(None);
		if VALIDATION_ENABLED
		{
			self.instance.destroy_debug_utils_messenger_ext(self.data.messenger, None);
		}
		self.instance.destroy_surface_khr(self.data.surface, None);
		self.instance.destroy_instance(None);
	}
}

pub unsafe fn create_render_pass(instance: &Instance, 
										device: &Device,
										data: &mut AppData) -> Result<()>
{
	let color_attachment = vk::AttachmentDescription::builder()
															.format(data.swapchain_format)
															.samples(vk::SampleCountFlags::_1)
															.load_op(vk::AttachmentLoadOp::CLEAR)
															.store_op(vk::AttachmentStoreOp::STORE)
															.stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
															.stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
															.initial_layout(vk::ImageLayout::UNDEFINED)
															.final_layout(vk::ImageLayout::PRESENT_SRC_KHR);
	let color_attachment_ref = vk::AttachmentReference::builder()
															.attachment(0)
															.layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
	let color_attachments = &[color_attachment_ref];
	let subpass = vk::SubpassDescription::builder()
													.pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
													.color_attachments(color_attachments);
	let attachments = &[color_attachment];
	let subpasses = &[subpass];
	let info = vk::RenderPassCreateInfo::builder()
												.attachments(attachments)
												.subpasses(subpasses);
	data.render_pass = device.create_render_pass(&info, None)?;

	Ok(())
}

/// The Vulkan handles and associated properties used by our Vulkan app.
#[derive(Clone, Debug, Default)]
pub struct AppData {
	pub messenger: vk::DebugUtilsMessengerEXT,
	pub physical_device: vk::PhysicalDevice,
	pub graphics_queue: vk::Queue,
	pub surface: vk::SurfaceKHR,
	pub present_queue : vk::Queue,
	pub swapchain : vk::SwapchainKHR,
	pub swapchain_format: vk::Format,
	pub swapchain_extent: vk::Extent2D,
	pub swapchain_images: Vec<vk::Image>,
	pub swapchain_image_views: Vec<vk::ImageView>,
	pub render_pass: vk::RenderPass,
	pub pipeline_layout: vk::PipelineLayout,
	pub pipeline: vk::Pipeline
}

#[derive(Debug, Error)]
#[error("Missing {0}.")]
pub struct SuitabilityError(pub &'static str);

#[derive(Copy, Clone, Debug)]
pub struct QueueFamilyIndices {
    pub graphics: u32,
	pub present: u32
}

impl QueueFamilyIndices
{
	pub unsafe fn get(instance: &Instance, data: &AppData , physical_device: vk::PhysicalDevice) -> Result<Self>
	{
		let properties = instance.get_physical_device_queue_family_properties(physical_device);
		let graphics = properties.iter()
																	.position(|p| p.queue_flags.contains(vk::QueueFlags::GRAPHICS))
																	.map(|i| i as u32);
		let mut present = None;
		for(index, properties) in properties.iter().enumerate(){
			if instance.get_physical_device_surface_support_khr(physical_device, index as u32, data.surface)?
			{
				present = Some(index as u32);
				break;
			}
		}
		if let (Some(graphics), Some(present)) = (graphics, present) {
			Ok(Self { graphics, present })
		}
		else {
			Err(anyhow!(SuitabilityError("Missing required queue families.")))
		}
	}
}


#[derive(Clone, Debug)]
pub struct SwapchainSupport
{
	pub capabilites: vk::SurfaceCapabilitiesKHR,
	pub formats: Vec<vk::SurfaceFormatKHR>,
	pub present_modes: Vec<vk::PresentModeKHR>	
}

impl SwapchainSupport {
	pub unsafe fn get(instance: &Instance,
						data: &AppData,
						physical_device: vk::PhysicalDevice) -> Result<Self>
	{
		Ok(Self { capabilites: instance.get_physical_device_surface_capabilities_khr(physical_device, data.surface)?, 
					formats: instance.get_physical_device_surface_formats_khr(physical_device, data.surface)?, 
					present_modes: instance.get_physical_device_surface_present_modes_khr(physical_device, data.surface)? 
				})
	}
}


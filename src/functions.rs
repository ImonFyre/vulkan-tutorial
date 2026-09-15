use anyhow::{
	Ok, 
	Result, 
	anyhow
};
use log::*;
use std::{
	collections::HashSet, 
	ffi::CStr,
	os::raw::c_void
};
use vulkanalia::{
	Version, 
	bytecode::Bytecode, 
	prelude::v1_0::*, 
	vk::{ 
		ExtDebugUtilsExtensionInstanceCommands, 
		KhrSwapchainExtensionDeviceCommands 
	}, 
	window as vk_window
};
use winit::window::Window;


use crate::structs::{
	AppData, 
	QueueFamilyIndices, 
	SuitabilityError, 
	SwapchainSupport
};


pub const VALIDATION_ENABLED : bool = cfg!(debug_assertions);
const VALIDATION_LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");
const PORTABILITY_MACOS_VERSION: Version = Version::new(1,3,216);
const DEVICE_EXTENSTIONS : &[vk::ExtensionName] = &[vk::KHR_SWAPCHAIN_EXTENSION.name];


pub unsafe fn create_instance(window: &Window, entry: &Entry, data: &mut AppData) -> Result<Instance> 
{
	let application_info = vk::ApplicationInfo::builder()
		.application_name(b"Vulkan Tutorial\0")
		.application_version(vk::make_version(1, 0, 0))
		.engine_name(b"No Engine\0")
		.engine_version(vk::make_version(1, 0, 0))
		.api_version(vk::make_version(1, 0, 0));

	let available_layers = entry.enumerate_instance_layer_properties()?
																.iter()
																.map(|l| l.layer_name)
																.collect::<HashSet<_>>();

	if VALIDATION_ENABLED && !available_layers.contains(&VALIDATION_LAYER)
	{
		return Err(anyhow!("Validation layer requested but not supported - 1.\n{:?}", available_layers));
	}

	let layers = if VALIDATION_ENABLED {
		vec![VALIDATION_LAYER.as_ptr()]
	}
	else {
		Vec::new()
	};


	let mut extensions = vk_window::get_required_instance_extensions(window)
		.iter()
		.map(|e| e.as_ptr())
		.collect::<Vec<_>>();
	
	if VALIDATION_ENABLED
	{
		extensions.push(vk::EXT_DEBUG_UTILS_EXTENSION.name.as_ptr());
	}

	let flags = if cfg!(target_os = "macos")
						&& entry.version()? == PORTABILITY_MACOS_VERSION
						{
							info!("Enabling extensions for macOS portability");
							extensions.push(vk::KHR_GET_PHYSICAL_DEVICE_PROPERTIES2_EXTENSION.name.as_ptr());
							extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
							vk::InstanceCreateFlags::ENUMERATE_PORTABILITY_KHR
						} 
						else 
						{
							vk::InstanceCreateFlags::empty()
						};

	
	let mut info = vk::InstanceCreateInfo::builder()
		.application_info(&application_info)
		.enabled_layer_names(&layers)
		.enabled_extension_names(&extensions)
		.flags(flags);

	let mut debug_info = vk::DebugUtilsMessengerCreateInfoEXT::builder()
					.message_severity(vk::DebugUtilsMessageSeverityFlagsEXT::all())
					.message_type(vk::DebugUtilsMessageTypeFlagsEXT::GENERAL  
									| vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION 
									| vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE)
					.user_callback(Some(debug_callback));
	if VALIDATION_ENABLED
	{
		info = info.push_next(&mut debug_info);
	}

	let instance = entry.create_instance(&info, None)?;
	if VALIDATION_ENABLED
	{
		data.messenger = instance.create_debug_utils_messenger_ext(&debug_info, None)?;
	}

	Ok(instance)
}


pub unsafe fn create_logical_device(entry: &Entry, instance: &Instance, data: &mut AppData) -> Result<Device>
{
		let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
		
		let mut unique_indices = HashSet::new();
		unique_indices.insert(indices.graphics);
		unique_indices.insert(indices.present);


		let queue_priorities = &[1.0];
		let queue_infos = unique_indices.iter()
														.map(|i| { vk::DeviceQueueCreateInfo::builder()
																.queue_family_index(*i)
															.queue_priorities(queue_priorities)
														})
														.collect::<Vec<_>>();
													
		let queue_info = vk::DeviceQueueCreateInfo::builder()
																.queue_family_index(indices.graphics)
																.queue_priorities(queue_priorities);

		let layers = if VALIDATION_ENABLED {
			vec![VALIDATION_LAYER.as_ptr()]
		} 
		else
		{
			vec![]
		};
		let mut extensions = DEVICE_EXTENSTIONS.iter()
																			.map(|n| n.as_ptr())
																			.collect::<Vec<_>>();
		if cfg!(target_os = "macos") && entry.version()? >= PORTABILITY_MACOS_VERSION
		{
			extensions.push(vk::KHR_PORTABILITY_ENUMERATION_EXTENSION.name.as_ptr());
		}

		let features = vk::PhysicalDeviceFeatures::builder();

		let info = vk::DeviceCreateInfo::builder()
													.queue_create_infos(&queue_infos)
													//.enabled_layer_names(&layers)
													.enabled_extension_names(&extensions)
													.enabled_features(&features);
		let device = instance.create_device(data.physical_device, &info, None)?;
		data.graphics_queue = device.get_device_queue(indices.graphics, 0);
		data.present_queue = device.get_device_queue(indices.present, 0);
		Ok(device)
	}

pub unsafe fn create_pipeline(device: &Device, data: &mut AppData) -> Result<()>
{
	let vert = include_bytes!("../shaders/vert.spv");
	let frag = include_bytes!("../shaders/frag.spv");
	let vert_shader_module = create_shader_module(device, &vert[..])?;
	let frag_shader_module = create_shader_module(device, &frag[..])?;

	let vert_stage = vk::PipelineShaderStageCreateInfo::builder()
					.stage(vk::ShaderStageFlags::VERTEX)
					.name(b"main\0");

	let frag_stage = vk::PipelineShaderStageCreateInfo::builder()
						.stage(vk::ShaderStageFlags::FRAGMENT)
						.name(b"main\0");

	let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();
	
	let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
								.topology(vk::PrimitiveTopology::TRIANGLE_LIST)
								.primitive_restart_enable(false);

	let viewport = vk::Viewport::builder()
						.x(0.0)
						.y(0.0)
						.width(data.swapchain_extent.width as f32)
						.height(data.swapchain_extent.height as f32)
						.min_depth(0.0)
						.max_depth(1.0);

	let scissor = vk::Rect2D::builder()
								.offset(vk::Offset2D {x:0, y:0})
								.extent(data.swapchain_extent);
	
	let viewports = &[viewport];
	let scissors = &[scissor];
	let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
							.viewports(viewports)
							.scissors(scissors);
	
	let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
								.depth_clamp_enable(false)
								.rasterizer_discard_enable(false)
								.polygon_mode(vk::PolygonMode::FILL)
								.line_width(1.0)
								.cull_mode(vk::CullModeFlags::BACK)
								.front_face(vk::FrontFace::CLOCKWISE)
								.depth_bias_enable(false);
	
	let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
						.sample_shading_enable(false)
						.rasterization_samples(vk::SampleCountFlags::_1);

	
	let attachement = vk::PipelineColorBlendAttachmentState::builder()
					.color_write_mask(vk::ColorComponentFlags::all())
					.blend_enable(false)
					.src_color_blend_factor(vk::BlendFactor::ONE)
					.dst_color_blend_factor(vk::BlendFactor::ZERO)
					.color_blend_op(vk::BlendOp::ADD)
					.src_alpha_blend_factor(vk::BlendFactor::ONE)
					.dst_alpha_blend_factor(vk::BlendFactor::ZERO)
					.alpha_blend_op(vk::BlendOp::ADD);
	let attachments = &[attachement];

	let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
																		.logic_op_enable(false)
																		.logic_op(vk::LogicOp::COPY)
																		.attachments(attachments)
																		.blend_constants([0.0,0.0,0.0,0.0]);
	
	let layout_info = vk::PipelineLayoutCreateInfo::builder();
	
	data.pipeline_layout = device.create_pipeline_layout(&layout_info, None)?;

	device.destroy_shader_module(vert_shader_module, None);
	device.destroy_shader_module(frag_shader_module, None);

	let stages = &[vert_stage, frag_stage];
	let info = vk::GraphicsPipelineCreateInfo::builder()
														.stages(stages)
														.vertex_input_state(&vertex_input_state)
														.input_assembly_state(&input_assembly_state)
														.viewport_state(&viewport_state)
														.rasterization_state(&rasterization_state)
														.multisample_state(&multisample_state)
														.color_blend_state(&color_blend_state)
														.layout(data.pipeline_layout)
														.render_pass(data.render_pass)
														.subpass(0)
														.base_pipeline_handle(vk::Pipeline::null())
														.base_pipeline_index(-1);


	data.pipeline = device.create_graphics_pipelines(vk::PipelineCache::null(),
														&[info], 
														None)?
							.0[0];
	Ok(())
}

pub unsafe fn create_shader_module(device: &Device, bytecode: &[u8]) -> Result<vk::ShaderModule>
{
	let bytecode = Bytecode::new(bytecode).unwrap();
	let info = vk::ShaderModuleCreateInfo::builder()
									.code(bytecode.code())
									.code_size(bytecode.code_size());
	
	Ok(device.create_shader_module(&info, None)?)
}

pub unsafe fn create_swapchain(window: &Window, instance: &Instance, device: &Device, data: &mut AppData) -> Result<()> 
{
	let indices = QueueFamilyIndices::get(instance, data, data.physical_device)?;
	let support = SwapchainSupport::get(instance, data, data.physical_device)?;
	let surface_format = get_swapchain_surface_format(&support.formats);
	let present_mode = get_swapchain_present_mode(&support.present_modes);
	let extent = get_swapchain_extent(window, support.capabilites);
	
	let mut image_count = support.capabilites.min_image_count + 1;
	if support.capabilites.max_image_count != 0
		&& image_count > support.capabilites.max_image_count
	{
		image_count = support.capabilites.max_image_count;
	}

	let mut queue_family_indices = vec![];
	let image_sharing_mode = if indices.graphics != indices.present {
		queue_family_indices.push(indices.graphics);
		queue_family_indices.push(indices.present);
		vk::SharingMode::CONCURRENT
	}
	else 
	{
		vk::SharingMode::EXCLUSIVE
	};

	let info = vk::SwapchainCreateInfoKHR::builder()
							.surface(data.surface)
							.min_image_count(image_count)
							.image_format(surface_format.format)
							.image_color_space(surface_format.color_space)
							.image_extent(extent)
							.image_array_layers(1)
							.image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
							.image_sharing_mode(image_sharing_mode)
							.queue_family_indices(&queue_family_indices)
							.pre_transform(support.capabilites.current_transform)
							.composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
							.present_mode(present_mode)
							.clipped(true)
							.old_swapchain(vk::SwapchainKHR::null());
	data.swapchain = device.create_swapchain_khr(&info, None)?;
	data.swapchain_images = device.get_swapchain_images_khr(data.swapchain)?;
	data.swapchain_format = surface_format.format;
	data.swapchain_extent = extent;
	Ok(())
}

pub unsafe fn create_swapchain_image_views(device: &Device, data: &mut AppData) -> Result<()>
{
	data.swapchain_image_views = data
	.swapchain_images
	.iter()
	.map(|i| {
		let components = vk::ComponentMapping::builder()
							.r(vk::ComponentSwizzle::IDENTITY)
							.g(vk::ComponentSwizzle::IDENTITY)
							.b(vk::ComponentSwizzle::IDENTITY)
							.a(vk::ComponentSwizzle::IDENTITY);

		let subresource_range = vk::ImageSubresourceRange::builder()
									.aspect_mask(vk::ImageAspectFlags::COLOR)
									.base_mip_level(0)
									.level_count(1)
									.base_array_layer(0)
									.layer_count(1);

		let info = vk::ImageViewCreateInfo::builder()
						.image(*i)
						.view_type(vk::ImageViewType::_2D)
						.format(data.swapchain_format)
						.components(components)
						.subresource_range(subresource_range);
		device.create_image_view(&info, None)
	})
	.collect::<Result<Vec<_>,_>>()?;
	Ok(())
}



pub unsafe fn check_physical_device(instance: &Instance, data: &AppData, physical_device: vk::PhysicalDevice) -> Result<()>
{
	
	let properties = instance.get_physical_device_properties(physical_device);
	if properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU
	{
		return Err(anyhow!(SuitabilityError("Only discrete GPUs are supported!")));
	}

	let features = instance.get_physical_device_features(physical_device);
	if features.geometry_shader != vk::TRUE
	{
		return Err(anyhow!(SuitabilityError("Missing geometry shader support.")));
	}
	QueueFamilyIndices::get(instance, data, physical_device)?;
	check_physical_device_extentions(instance, physical_device)?;

	let support = SwapchainSupport::get(instance, data, physical_device)?;
	if support.formats.is_empty() || support.present_modes.is_empty()
	{
		return Err(anyhow!(SuitabilityError("Insufficient swapchain support.")));
	}

	Ok(())

}

pub unsafe fn check_physical_device_extentions(instance: &Instance, physical_device: vk::PhysicalDevice) -> Result<()>
{
	let extensions = instance.enumerate_device_extension_properties(physical_device, None)?
														.iter()
														.map(|e| e.extension_name)
														.collect::<HashSet<_>>();
	if DEVICE_EXTENSTIONS.iter().all(|e| extensions.contains(e))
	{
		Ok(())
	}
	else 
	{
		Err(anyhow!(SuitabilityError("Mission required device extensions.")))	
	}
	
}



pub unsafe fn pick_physical_device(instance: &Instance, data: &mut AppData) -> Result<()>
{
	for physical_device in instance.enumerate_physical_devices()?
	{
		let properties = instance.get_physical_device_properties(physical_device);

        if let Err(error) = check_physical_device(instance, data, physical_device) 
		{
        	warn!("Skipping physical device (`{}`): {}", properties.device_name, error);
        } 
		else 
		{
            info!("Selected physical device (`{}`).", properties.device_name);
            data.physical_device = physical_device;
            return Ok(());
        }
	}
	Err(anyhow!("Failed to find suitable physical device."))
}



fn get_swapchain_surface_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR
{
	formats.iter()
			.cloned()
			.find(|f| { f.format == vk::Format::B8G8R8A8_SRGB
											&& f.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR})
			.unwrap_or_else(|| formats[0])
}

fn get_swapchain_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR
{
	present_modes.iter()
					.cloned()
					.find(|f| *f == vk::PresentModeKHR::MAILBOX)
					.unwrap_or(vk::PresentModeKHR::FIFO)
}

fn get_swapchain_extent(window: &Window,
    					capabilities: vk::SurfaceCapabilitiesKHR) -> vk::Extent2D 
{
	if capabilities.current_extent.width != u32::MAX
	{
		capabilities.current_extent
	}
	else 
	{
		vk::Extent2D::builder()
						.width(window.inner_size().width.clamp(capabilities.min_image_extent.width,
																capabilities.max_image_extent.width))
						.height(window.inner_size().height.clamp(capabilities.min_image_extent.height,
																capabilities.max_image_extent.height))
						.build()

	}
}



extern "system" fn debug_callback(severity: vk::DebugUtilsMessageSeverityFlagsEXT,
									type_: vk::DebugUtilsMessageTypeFlagsEXT,
									data: *const vk::DebugUtilsMessengerCallbackDataEXT,
									_: *mut c_void) -> vk::Bool32
{
	let data = unsafe { *data };
	let message = unsafe { CStr::from_ptr(data.message)}.to_string_lossy();
	if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::ERROR 
	{
		error!("({:?}) {}", type_, message);
	}
	else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::WARNING 
	{
		warn!("({:?}) {}", type_, message);
	}
	else if severity >= vk::DebugUtilsMessageSeverityFlagsEXT::INFO 
	{
		debug!("({:?}) {}", type_, message);
	}
	else
	{
		trace!("({:?}) {}", type_, message);
	}
	vk::FALSE
}
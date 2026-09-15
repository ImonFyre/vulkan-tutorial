#![allow(
	dead_code,
	unsafe_op_in_unsafe_fn,
	unused_variables,
	clippy::too_many_arguments,
	clippy::unnecessary_wraps
)]

pub mod functions;
pub mod structs;

use crate::structs::App;

use anyhow::{
	Ok, 
	Result
};
use vulkanalia::{
	Version,
	prelude::v1_0::*, 
};
use winit::{ 
	dpi::LogicalSize,
	event::{
		Event,
		WindowEvent
	},
	event_loop::EventLoop,
	window::WindowBuilder
};


const VALIDATION_ENABLED : bool = cfg!(debug_assertions);
const VALIDATION_LAYER: vk::ExtensionName = vk::ExtensionName::from_bytes(b"VK_LAYER_KHRONOS_validation");
const PORTABILITY_MACOS_VERSION: Version = Version::new(1,3,216);


fn main() -> Result<()> {
	pretty_env_logger::init();

	// Window

	let event_loop = EventLoop::new()?;
	let window = WindowBuilder::new()
		.with_title("Vulkan Tutorial (Rust)")
		.with_inner_size(LogicalSize::new(1024, 768))
		.build(&event_loop)?;

	// App

	let mut app = unsafe { App::create(&window)? };
	event_loop.run(move |event, elwt| {
		match event {
			// Request a redraw when all events were processed.
			Event::AboutToWait => window.request_redraw(),
			Event::WindowEvent { event, .. } => match event {
				// Render a frame if our Vulkan app is not being destroyed.
				WindowEvent::RedrawRequested if !elwt.exiting() => {
					unsafe { app.render(&window) }.unwrap()
				}
				// Destroy our Vulkan app.
				WindowEvent::CloseRequested => {
					elwt.exit();
					unsafe {
						app.destroy();
					}
				}
				_ => {}
			},
			_ => {}
		}
	})?;

	Ok(())
}








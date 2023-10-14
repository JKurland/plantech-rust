use message_proc_macros::Message;

#[derive(Message, Debug)]
#[pt_response(winit::window::WindowId)]
pub struct OpenWindow {
    pub title: String,
    pub width: u32,
    pub height: u32,
}


#[derive(Message, Debug)]
#[pt_response(())]
pub struct CloseWindow {
    pub window: winit::window::WindowId,
}


#[derive(Message, Debug)]
#[pt_response(())]
pub struct ExitProgram {
    pub code: u8,
}


#[derive(Message, Debug, Clone)]
pub struct KeyPress {
    pub key: winit::event::VirtualKeyCode,
    pub state: winit::event::ElementState,
}

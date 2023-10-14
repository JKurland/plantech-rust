use std::{thread::JoinHandle, collections::HashMap, cell::{Cell, OnceCell}};

use futures::{FutureExt, future::LocalBoxFuture};
use handler_proc_macros::Handler;
use handler_structs::{HandlerInit, Handle};
use message_list::C;


use application_messages::*;
use shared_future::SharedFuture;
use winit::{event_loop::{EventLoop, EventLoopBuilder}, platform::wayland::EventLoopBuilderExtWayland, event};

mod shared_future;

#[derive(Debug)]
enum WinitEvent {
    OpenWindow((OpenWindow, smol::channel::Sender<winit::window::WindowId>)),
    CloseWindow(CloseWindow),
}

struct EventLoopData {
    event_loop_proxy: winit::event_loop::EventLoopProxy<WinitEvent>,
    event_loop_join_handle: JoinHandle<()>,
}

#[derive(Handler)]
#[pt_handles(OpenWindow, CloseWindow)]
pub struct Windows {
    event_loop_data: OnceCell<SharedFuture<EventLoopData>>,
}

impl HandlerInit for Windows {
    fn init<'a, Ctx: C + 'a>(_ctx: &Self::InitCtx<'a, Ctx>, _config: Self::InitConfig) -> Self {
        Self {event_loop_data: OnceCell::new()}
    }
}

impl Handle<OpenWindow> for Windows {
    fn handle<'a>(&'a self, ctx: &'a impl C, message: OpenWindow) -> <OpenWindow as message_structs::Message>::Response<'a> {
        async move {
            let event_loop_data = self.event_loop_data.get_or_init(|| Self::make_event_loop_data(ctx)).clone().await;

            let (sender, receiver) = smol::channel::bounded(1);
            event_loop_data.event_loop_proxy.send_event(WinitEvent::OpenWindow((message, sender))).unwrap();
            receiver.recv().await.unwrap()
        }.boxed_local()
    }
}

impl Handle<CloseWindow> for Windows {
    fn handle<'a>(&'a self, ctx: &'a impl C, message: CloseWindow) -> <CloseWindow as message_structs::Message>::Response<'a> {
        async move {
            let event_loop_data = self.event_loop_data.get_or_init(|| Self::make_event_loop_data(ctx)).clone().await;

            event_loop_data.event_loop_proxy.send_event(WinitEvent::CloseWindow(message)).unwrap();
        }.boxed_local()
    }
}

impl Windows {
    fn run(event_loop: EventLoop<WinitEvent>, ctx_proxy: Box<dyn C>) {
        let mut windows = HashMap::new();

        event_loop.run(move |event, window_target, control_flow| {
            control_flow.set_wait();
            match event {
                winit::event::Event::WindowEvent { event, window_id } => {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            drop(ctx_proxy.handle(CloseWindow{window: window_id}));
                        },
                        winit::event::WindowEvent::KeyboardInput {input, ..} => {
                            let virtual_keycode = input.virtual_keycode;
                            let state = input.state;
                            if let Some(virtual_keycode) = virtual_keycode {
                                // don't want to wait for the handlers to finish so drop the future
                                drop(ctx_proxy.handle(KeyPress{key: virtual_keycode, state}));
                            }
                        },
                        _ => (),
                    }
                },
                winit::event::Event::UserEvent(event) => {
                    match event {
                        WinitEvent::OpenWindow((event, resp)) => {
                            let window = winit::window::WindowBuilder::new()
                                .with_title(event.title)
                                .with_inner_size(winit::dpi::LogicalSize::new(event.width, event.height))
                                .build(window_target)
                                .unwrap();
                            let id = window.id();
                            windows.insert(id, window);
                            resp.send_blocking(id).unwrap();
                        },
                        WinitEvent::CloseWindow(event) => {
                            windows.remove(&event.window);
                        },
                    }
                },
                _ => (),
            }
        })
    }

    fn make_event_loop_data(ctx: &impl C) -> SharedFuture<EventLoopData> {
        let ctx_proxy = ctx.proxy();
        let fut = async {
            let (sender, receiver) = smol::channel::bounded(1);
            let event_loop_join_handle = std::thread::spawn(move || {
                let event_loop = EventLoopBuilder::<WinitEvent>::with_user_event()
                    .with_any_thread(true)
                    .build();
                
                let event_loop_proxy = event_loop.create_proxy();
                sender.send_blocking(event_loop_proxy).unwrap();
                Self::run(event_loop, ctx_proxy);
            });

            let event_loop_proxy = receiver.recv().await.unwrap();
            EventLoopData{event_loop_proxy, event_loop_join_handle}
        };
        SharedFuture::new(fut.boxed_local())
    }
}

#[derive(Handler)]
#[pt_handles(ExitProgram)]
pub struct ExitHandler {}

impl HandlerInit for ExitHandler {
    fn init<'a, Ctx: C + 'a>(_ctx: &Self::InitCtx<'a, Ctx>, _config: Self::InitConfig) -> Self {
        Self {}
    }
}


impl Handle<ExitProgram> for ExitHandler {
    fn handle<'a>(&'a self, _ctx: &'a impl C, message: ExitProgram) -> <ExitProgram as message_structs::Message>::Response<'a> {
        std::process::exit(message.code as i32);
    }
}
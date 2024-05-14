#![cfg(android_platform)]

use winit::event_loop::EventLoop;
use winit::platform::android::EventLoopBuilderExtAndroid;

#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let event_loop = EventLoop::builder().with_android_app(app).build().unwrap();
    glutin_examples::main(event_loop).unwrap()
}

#![cfg(android_platform)]

use winit::event_loop::EventLoopBuilder;
use winit::platform::android::EventLoopBuilderExtAndroid;

#[no_mangle]
fn android_main(app: winit::platform::android::activity::AndroidApp) {
    let event_loop = EventLoopBuilder::new().with_android_app(app).build().unwrap();
    glutin_examples::main(event_loop).unwrap()
}

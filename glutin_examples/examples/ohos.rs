#![cfg(ohos_platform)]

use openharmony_ability::OpenHarmonyApp;
use openharmony_ability_derive::ability;

use winit::event_loop::EventLoop;
use winit::platform::ohos::EventLoopBuilderExtOpenHarmony;

#[ability]
pub fn openharmony(openharmony_app: OpenHarmonyApp) {
    let a = openharmony_app.clone();
    let event_loop = EventLoop::builder().with_openharmony_app(a).build().unwrap();
    glutin_examples::main(event_loop).unwrap()
}

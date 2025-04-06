use app::App;
use window::GameWindow;
use winit::event_loop::EventLoop;

mod app;
mod window;

fn main() {
    pretty_env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    let mut window = GameWindow::<App>::new("My app");

    event_loop.run_app(&mut window)
        .expect("Error occured while running application");
}

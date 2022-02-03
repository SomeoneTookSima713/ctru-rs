use ctru::console::Console;
use ctru::services::hid::KeyPad;
use ctru::services::{Apt, Hid};
use ctru::Gfx;
use std::time::Duration;

fn main() {
    ctru::init();
    let gfx = Gfx::default();
    let hid = Hid::init().expect("Couldn't obtain HID controller");
    let apt = Apt::init().expect("Couldn't obtain APT controller");
    let _console = Console::init(gfx.top_screen.borrow_mut());

    // FIXME: replace this with `Ps` when #39 merges
    assert!(unsafe { ctru_sys::psInit() } >= 0);

    // Give ourselves up to 30% of the system core's time
    apt.set_app_cpu_time_limit(30)
        .expect("Failed to enable system core");

    println!("Starting runtime...");

    let (exit_sender, mut exit_receiver) = tokio::sync::oneshot::channel();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("Couldn't build runtime");

    let runtime_thread = ctru::thread::Builder::new()
        .affinity(1)
        .spawn(move || {
            runtime.block_on(async move {
                let mut wake_time = tokio::time::Instant::now() + Duration::from_secs(1);
                loop {
                    let sleep_future = tokio::time::sleep_until(wake_time);

                    tokio::select! {
                        _ = &mut exit_receiver => break,
                        _ = sleep_future => {
                            println!("Tick");
                            wake_time += Duration::from_secs(1);
                        }
                    }
                }
            });
        })
        .expect("Failed to create runtime thread");

    println!("Runtime started!");

    while apt.main_loop() {
        hid.scan_input();

        if hid.keys_down().contains(KeyPad::KEY_START) {
            println!("Shutting down...");
            let _ = exit_sender.send(());
            let _ = runtime_thread.join();
            break;
        }

        gfx.flush_buffers();
        gfx.swap_buffers();
        gfx.wait_for_vblank();
    }
}

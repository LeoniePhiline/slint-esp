use core::sync::atomic::Ordering;

use portable_atomic::{AtomicBool, AtomicF32, AtomicI64};

slint::include_modules!();

mod dht22;
mod esp32;

static HAVE_DATA: AtomicBool = AtomicBool::new(false);
static TEMPERATURE: AtomicF32 = AtomicF32::new(0.0);
static HUMIDITY: AtomicF32 = AtomicF32::new(0.0);
static TIMESTAMP: AtomicI64 = AtomicI64::new(0);

unsafe extern "C" fn dht_task(_: *mut core::ffi::c_void) {
    let dht = dht22::DHT22::new(13);

    loop {
        match dht.read() {
            #[allow(unused_variables)]
            Ok((temperature, humidity)) => {
                HAVE_DATA.store(true, Ordering::Relaxed);
                TEMPERATURE.store(temperature, Ordering::Relaxed);
                HUMIDITY.store(humidity, Ordering::Relaxed);
                log::info!("Temperature: {temperature:.2} °C, Humidity: {humidity:.2} %");
            }
            Err(err) => {
                HAVE_DATA.store(false, Ordering::Relaxed);
                log::error!("Error reading DHT22: {err:?}");
            }
        }

        // Store time in microseconds since boot.
        TIMESTAMP.store(esp_idf_svc::sys::esp_timer_get_time(), Ordering::Relaxed);

        esp_idf_svc::sys::vTaskDelay(2000 / 10);
    }
}

fn main() {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    // Set the platform
    slint::platform::set_platform(esp32::EspPlatform::new()).unwrap();

    // Create DHT task
    unsafe {
        let mut task_handle = std::ptr::null_mut();
        esp_idf_svc::sys::xTaskCreatePinnedToCore(
            Some(dht_task),
            b"sensor_task\0".as_ptr() as *const i8,
            4096,
            std::ptr::null_mut(),
            5,
            &mut task_handle,
            1,
        );
    }

    // Finally, run the app!
    let ui = AppWindow::new().expect("Failed to load UI");

    ui.on_request_increase_value({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    let timer = slint::Timer::default();
    timer.start(
        slint::TimerMode::Repeated,
        std::time::Duration::from_millis(2000),
        {
            let ui_handle = ui.as_weak();
            move || {
                let ui = ui_handle.unwrap();

                let have_data = HAVE_DATA.load(Ordering::Relaxed);

                ui.global::<ViewModel>().set_have_data(have_data);

                if !have_data {
                    return;
                }

                ui.global::<ViewModel>().set_weather(WeatherRecord {
                    humidity: HUMIDITY.load(Ordering::Relaxed),
                    temperature: TEMPERATURE.load(Ordering::Relaxed),
                    // Format millisecond-resolution timestamp  as seconds.
                    timestamp: slint::format!(
                        "{:?}s",
                        TIMESTAMP.load(Ordering::Relaxed) / 1_000_000
                    ),
                });

                HAVE_DATA.store(false, Ordering::Relaxed);
            }
        },
    );

    ui.run().unwrap();
}

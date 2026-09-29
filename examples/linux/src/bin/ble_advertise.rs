use trouble_example_apps::ble_advertise;
use trouble_linux_examples::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(ble_advertise::run)
}

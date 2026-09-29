use trouble_example_apps::ble_bas_peripheral_sec;
use trouble_linux_examples::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(ble_bas_peripheral_sec::run)
}

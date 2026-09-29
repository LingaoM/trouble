use trouble_example_apps::ble_scanner;
use trouble_linux_examples::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(ble_scanner::run)
}

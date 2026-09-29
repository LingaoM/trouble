use trouble_example_apps::ble_l2cap_central;
use trouble_linux_examples::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(ble_l2cap_central::run)
}

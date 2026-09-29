use trouble_example_apps::{high_throughput_ble_l2cap_central, BigAlloc};
use trouble_linux_examples::run;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(high_throughput_ble_l2cap_central::run::<_, BigAlloc>)
}

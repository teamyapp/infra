mod platform_testing;
mod platform;
mod tests;

fn main() {
    println!("Run 'cargo test' to execute the tests.");
    platform_testing::network::SimulatedNetwork::new(0.1, 0.2);
}
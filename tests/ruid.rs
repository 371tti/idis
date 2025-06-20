use idis::utils::ruid::RUIDGenerator;

#[test]
fn test_ruid_generation() {
    let mut generator = RUIDGenerator::new_with_entropy(42);
    generator.set_device_id(1);
    let ruid = generator.generate(2);
    println!("Generated RUID: {:?}", ruid);
    assert_eq!(ruid.get_device_id(), 1);
    assert_eq!(ruid.get_prefix(), 2);
}
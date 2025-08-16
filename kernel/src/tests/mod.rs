// Comprehensive Hardware Testing Module
#![cfg(test)]

pub mod sound_tests;
pub mod nvme_tests;
pub mod pcie_tests;
pub mod integration_tests;
pub mod tcp_tests;
pub mod tcp_stress_tests;

use crate::{serial_print, serial_println};

pub trait Testable {
    fn run(&self) -> ();
}

impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        serial_print!("{}...\t", core::any::type_name::<T>());
        self();
        serial_println!("[ok]");
    }
}

#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    serial_println!("Running {} tests", tests.len());
    for test in tests {
        test.run();
    }
    serial_println!("All tests passed!");
}

#[test_case]
fn trivial_assertion() {
    assert_eq!(1, 1);
}
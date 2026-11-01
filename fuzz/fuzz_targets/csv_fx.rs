#![no_main]

libfuzzer_sys::fuzz_target!(|data: &[u8]| finalytics_fuzz_checks::csv_fx(data));

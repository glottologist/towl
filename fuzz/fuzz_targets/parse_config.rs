#![no_main]
use libfuzzer_sys::fuzz_target;
use towl::config::TowlConfig;
use tempfile::NamedTempFile;
use std::io::Write;

fuzz_target!(|data: &[u8]| {
    if let Ok(config_content) = std::str::from_utf8(data) {
        if let Ok(mut temp_file) = NamedTempFile::new() {
            let _ = temp_file.write_all(config_content.as_bytes());
            let _ = temp_file.flush();

            let path = temp_file.path().to_path_buf();
            let _ = TowlConfig::load(Some(&path));
        }
    }
});

// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

pub mod shell;
pub mod thread;

pub use shell::Shell;

#[derive(Clone, Copy)]
pub struct SourceLocation {
    pub file_name: &'static str,
    pub module_path: &'static str,
    pub line: u32,
}

#[macro_export]
macro_rules! source_location {
    () => {
        $crate::klib::diag::SourceLocation {
            file_name: file!(),
            module_path: module_path!(),
            line: line!(),
        }
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn source_location_macro_captures_its_call_site() {
        let expected_line = line!() + 1;
        let location = crate::source_location!();

        assert_eq!(location.file_name, file!());
        assert_eq!(location.module_path, module_path!());
        assert_eq!(location.line, expected_line);
    }
}

// Copyright 2026 Cole Francis
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! # file_io
//!
//! handles reading and writing of simulator input and output files
//!
//! ## Invariants
//!
//! Author: Cole Francis

use std::fs;
use std::path::PathBuf;

use super::IoVal;
use super::runtime_diagnostics::{RuntimeError, IoError};

#[derive(Debug, PartialEq)]
enum FileType {
    Text,
    Invalid,
}

pub struct IoFile;

impl IoFile {
    pub fn read(file_path: &str) -> Result<Vec<(String, Vec<(usize, IoVal)>)>, RuntimeError> {
        let path = PathBuf::from(file_path);

        let file_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("txt") => FileType::Text,
            _ => FileType::Invalid,
        };

        if file_type == FileType::Invalid {
            return Err(RuntimeError::InvalidInputFileType);
        }

        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(err) => return Err(RuntimeError::Io(IoError(err)))
        };

        match file_type {
            FileType::Text => Self::read_txt_file(&code),
            _ => Err(RuntimeError::InvalidInputFileType),
        }
    }

    fn read_txt_file(code: &str) -> Result<Vec<(String, Vec<(usize, IoVal)>)>, RuntimeError> {
        
        for (line_num, line) in code.lines().enumerate() {
            let line_num = line_num + 1;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();

            if line_num == 1 {
                if fields != vec!["step", "entity", "value"] {
                    // TODO: Runtime error
                }
            }

            if fields.len() != 3 {
                // TODO: Runtime error
            }
        }
        Ok(vec![]) // Temporary
    }
}
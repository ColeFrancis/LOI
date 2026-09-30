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
use std::collections::HashMap;

use super::IoVal;
use super::runtime_diagnostics::{InputFileParseError, IoError};

#[derive(Debug, PartialEq)]
enum FileType {
    Text,
    Invalid,
}

pub struct IoFile;

impl IoFile {
    pub fn read(file_path: &str) -> Result<Vec<(String, Vec<(usize, IoVal)>)>, InputFileParseError> {
        let path = PathBuf::from(file_path);

        let file_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("txt") => FileType::Text,
            _ => FileType::Invalid,
        };

        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(err) => return Err(InputFileParseError::Io(IoError(err)))
        };

        match file_type {
            FileType::Text => Self::read_txt_file(&code),
            _ => Err(InputFileParseError::InvalidInputFileType),
        }
    }

    pub fn write(file_path: &str, outputs: Vec<(String, Vec<(usize, IoVal)>)>) -> Result<(), OutputFileWriteError> {
        let path = PathBuf::from(file_path);

        let file_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("txt") => FileType::Text,
            _ => FileType::Invalid,
        };

        let code = match file_type {
            FileType::Text => Self::write_txt_file(outputs),
            FileType::Invalid => return Err(OutputFileWriteError::InvalidOutputFileType),
        };

        fs::write(&path, code)
            .map_err(|err| OutputFileWriteError::Io(IoError(err)))?;

        Ok(())
    }

    fn read_txt_file(code: &str) -> Result<Vec<(String, Vec<(usize, IoVal)>)>, InputFileParseError> {
        // Create hashmap to map from entity to the index of the returned vector
        let mut map_to_idx: HashMap<String, usize> = HashMap::new();
        let mut inputs: Vec<(String, Vec<(usize, IoVal)>)> = Vec::new();

        for (line_num, line) in code.lines().enumerate() {
            let line_num = line_num + 1;
            let line = line.trim();

            if line.is_empty() {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();

            if line_num == 1 {
                if fields != vec!["step", "entity", "value"] {
                    return Err(InputFileParseError::InvalidHeader);
                }
                continue;
            }

            if fields.len() != 3 {
                return Err(InputFileParseError::IncorrectNumberOfFields {
                    expected: 3,
                    found: fields.len(),
                    line_num,
                });
            }

            let step = fields[0].parse::<usize>()
                .map_err(|source| {
                    InputFileParseError::InvalidStep {
                        source,
                        line_num,
                    }
                })?;

            let entity = fields[1].to_string();

            let val = if Self::starts_with_number(fields[2]) {
                let num_str = fields[2];

                if num_str.matches('.').count() > 1 {
                    return Err(InputFileParseError::InvalidNumber {
                        num: num_str.to_string(),
                        line_num,
                    });
                }

                if num_str.contains('.') {
                    match num_str.parse::<f64>() {
                        Ok(x) => IoVal::Real(x),
                        Err(_) => {
                            return Err(InputFileParseError::InvalidNumber {
                                num: num_str.to_string(),
                                line_num,
                            });
                        }
                    } 
                } else {
                    match num_str.parse::<i64>() {
                        Ok(x) => IoVal::Int(x),
                        Err(_) => {
                            return Err(InputFileParseError::InvalidNumber {
                                num: num_str.to_string(),
                                line_num,
                            });
                        }
                    }
                }
            } else {
                match fields[2] {
                    "true" => IoVal::Bool(true),
                    "false" => IoVal::Bool(false),
                    other => IoVal::Custom(other.to_string()),
                }
            };

            let idx = if let Some(&idx) = map_to_idx.get(&entity) {
                idx
            }
            else {
                let idx = inputs.len();
                map_to_idx.insert(entity.clone(), idx);
                inputs.push((entity.to_string(), vec![]));
                idx
            };

            // .1 causes you to select the second element of the tuple
            inputs[idx].1.push((step, val));
        }

        Ok(inputs)
    }

    fn write_txt_file(outpus: Vec<(String, Vec<(usize, IoVal)>)>) -> Result<String, OutputFileWriteError> {
        // Make sure to sort outputs timewise
    }

    fn starts_with_number(s: &str) -> bool {
        s.chars().next().map_or(false, |c| c.is_numeric())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_errors() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0 B 1.0
            5 B false
            10 A true
            11 C this"
        );

        assert_eq!(result, Ok(vec![
            ("A".to_string(), vec![
                (0, IoVal::Int(0)),
                (10, IoVal::Bool(true)),
            ]),
            ("B".to_string(), vec![
                (0, IoVal::Real(1.0)),
                (5, IoVal::Bool(false)),
            ]),
            ("C".to_string(), vec![
                (11, IoVal::Custom("this".to_string())),
            ]),
        ]));
    }

    #[test]
    fn invalid_header() {
        let result = IoFile::read_txt_file(
            "steps entity value
            0 A 0
            0 B 1.0
            5 B false
            10 A true
            11 C this"
        );

        assert_eq!(result, Err(InputFileParseError::InvalidHeader));
    }

    #[test]
    fn incorrect_num_fields() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0 B 1.0
            5 B 
            10 A true
            11 C this"
        );

        assert_eq!(result, Err(InputFileParseError::IncorrectNumberOfFields {
            expected: 3,
            found: 2,
            line_num: 4,
        }));
    }

    #[test]
    fn invalid_step() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0.0 B 1.0
            5 B false
            10 A true
            11 C this"
        );

        assert!(matches!(result, Err(InputFileParseError::InvalidStep {..})));
    }

    #[test]
    fn invalid_number() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0 B 1.0.2
            5 B false
            10 A true
            11 C this"
        );

        assert_eq!(result, Err(InputFileParseError::InvalidNumber {
            num: "1.0.2".to_string(),
            line_num: 3,
        }));
    }
}
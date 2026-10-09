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

use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use super::IoVal;
use super::error_handling::io_file_error::{InputFileReadError, IoError, OutputFileWriteError};

#[derive(Debug, PartialEq)]
enum FileType {
    Text,
    Invalid,
}

pub struct IoFile;

impl IoFile {
    pub fn read(file_path: &str) -> Result<Vec<(String, Vec<(usize, IoVal)>)>, InputFileReadError> {
        let path = PathBuf::from(file_path);

        let file_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("txt") => FileType::Text,
            _ => FileType::Invalid,
        };

        let code = match fs::read_to_string(&path) {
            Ok(code) => code,
            Err(err) => return Err(InputFileReadError::Io(IoError(err))),
        };

        match file_type {
            FileType::Text => Self::read_txt_file(&code),
            _ => Err(InputFileReadError::InvalidInputFileType),
        }
    }

    pub fn write(
        file_path: &str,
        outputs: Vec<(String, Vec<(usize, IoVal)>)>,
    ) -> Result<(), OutputFileWriteError> {
        let path = PathBuf::from(file_path);

        let file_type = match path.extension().and_then(|ext| ext.to_str()) {
            Some("txt") => FileType::Text,
            _ => FileType::Invalid,
        };

        if file_type == FileType::Invalid {
            return Err(OutputFileWriteError::InvalidOutputFileType);
        }

        let file = fs::File::create(&path).map_err(|err| OutputFileWriteError::Io(IoError(err)))?;

        let mut writer = BufWriter::new(file);

        match file_type {
            FileType::Text => Self::write_txt_file(&mut writer, outputs)?,
            FileType::Invalid => unreachable!(),
        }

        writer
            .flush()
            .map_err(|err| OutputFileWriteError::Io(IoError(err)))?;

        Ok(())
    }

    fn read_txt_file(code: &str) -> Result<Vec<(String, Vec<(usize, IoVal)>)>, InputFileReadError> {
        // Create hashmap to map from entity to the index of the returned vector
        let mut map_to_idx: HashMap<String, usize> = HashMap::new();
        let mut inputs: Vec<(String, Vec<(usize, IoVal)>)> = Vec::new();

        for (line_num, line) in code.lines().enumerate() {
            let line_num = line_num + 1;
            let line = line.trim();

            if line.is_empty() || line.starts_with("//") {
                continue;
            }

            let fields: Vec<&str> = line.split_whitespace().collect();

            if line_num == 1 {
                if fields != vec!["step", "entity", "value"] {
                    return Err(InputFileReadError::InvalidHeader);
                }
                continue;
            }

            if fields.len() != 3 {
                return Err(InputFileReadError::IncorrectNumberOfFields {
                    expected: 3,
                    found: fields.len(),
                    line_num,
                });
            }

            let step = fields[0]
                .parse::<usize>()
                .map_err(|source| InputFileReadError::InvalidStep { source, line_num })?;

            let entity = fields[1].to_string();

            let val = if Self::starts_with_number(fields[2]) {
                let num_str = fields[2];

                if num_str.contains('.') {
                    match num_str.parse::<f64>() {
                        Ok(x) => IoVal::Float(x),
                        Err(_) => {
                            return Err(InputFileReadError::InvalidNumber {
                                num: num_str.to_string(),
                                line_num,
                            });
                        }
                    }
                } else {
                    match num_str.parse::<i64>() {
                        Ok(x) => IoVal::Int(x),
                        Err(_) => {
                            return Err(InputFileReadError::InvalidNumber {
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
            } else {
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

    fn write_txt_file<W: Write>(
        writer: &mut W,
        outputs: Vec<(String, Vec<(usize, IoVal)>)>,
    ) -> Result<(), OutputFileWriteError> {
        let mut timewise_outputs: Vec<(usize, String, IoVal)> = Vec::new();

        for (entity, traces) in outputs {
            for (step, value) in traces {
                timewise_outputs.push((step, entity.clone(), value));
            }
        }

        timewise_outputs.sort_by_key(|(step, _, _)| *step);

        writeln!(writer, "step entity value")
            .map_err(|err| OutputFileWriteError::Io(IoError(err)))?;

        for (step, entity, value) in timewise_outputs {
            writeln!(writer, "{step} {entity} {value}")
                .map_err(|err| OutputFileWriteError::Io(IoError(err)))?;
        }

        Ok(())
    }

    fn starts_with_number(s: &str) -> bool {
        let mut chars = s.chars();

        match chars.next() {
            Some('-' | '+') => chars.next().is_some_and(|c| c.is_ascii_digit()),
            Some(c) => c.is_ascii_digit(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn invalid_header() {
        let result = IoFile::read_txt_file(
            "steps entity value
            0 A 0
            0 B 1.0
            5 B false
            10 A true
            11 C this",
        );

        assert_eq!(result, Err(InputFileReadError::InvalidHeader));
    }

    #[test]
    fn incorrect_num_fields() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0 B 1.0
            5 B 
            10 A true
            11 C this",
        );

        assert_eq!(
            result,
            Err(InputFileReadError::IncorrectNumberOfFields {
                expected: 3,
                found: 2,
                line_num: 4,
            })
        );
    }

    #[test]
    fn invalid_step() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0.0 B 1.0
            5 B false
            10 A true
            11 C this",
        );

        assert!(matches!(
            result,
            Err(InputFileReadError::InvalidStep { .. })
        ));
    }

    #[test]
    fn invalid_number() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0 B 1.0.2
            5 B false
            10 A true
            11 C this",
        );

        assert_eq!(
            result,
            Err(InputFileReadError::InvalidNumber {
                num: "1.0.2".to_string(),
                line_num: 3,
            })
        );
    }

    #[test]
    fn no_errors() {
        let result = IoFile::read_txt_file(
            "step entity value
            0 A 0
            0 B 1.0
            5 B false
            10 A true
            11 C this",
        );

        assert_eq!(
            result,
            Ok(vec![
                (
                    "A".to_string(),
                    vec![(0, IoVal::Int(0)), (10, IoVal::Bool(true)),]
                ),
                (
                    "B".to_string(),
                    vec![(0, IoVal::Float(1.0)), (5, IoVal::Bool(false)),]
                ),
                (
                    "C".to_string(),
                    vec![(11, IoVal::Custom("this".to_string())),]
                ),
            ])
        );
    }

    #[test]
    fn write_then_read() {
        let outputs = vec![
            (
                "bool".to_string(),
                vec![(0, IoVal::Bool(true)), (1, IoVal::Bool(false))],
            ),
            (
                "int".to_string(),
                vec![(0, IoVal::Int(42)), (1, IoVal::Int(-123))],
            ),
            (
                "float".to_string(),
                vec![(0, IoVal::Float(3.14159)), (1, IoVal::Float(-0.25))],
            ),
            (
                "custom".to_string(),
                vec![
                    (0, IoVal::Custom("foo".to_string())),
                    (1, IoVal::Custom("bar".to_string())),
                ],
            ),
        ];

        let path = std::env::temp_dir().join("loi_test.txt");
        let path_str = path.to_str().unwrap();

        IoFile::write(path_str, outputs.clone()).unwrap();

        let read_inputs = IoFile::read(path_str).unwrap();

        fs::remove_file(path).unwrap();

        assert_eq!(outputs, read_inputs);
    }
}

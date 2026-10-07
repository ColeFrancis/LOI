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

//! # io_file_error
//!
//! Handles runtime errors in the simulator
//!
//! Author: Cole Francis

#[derive(Debug, PartialEq)]
pub enum InputFileReadError {
    InvalidInputFileType,
    Io(IoError),
    InvalidHeader,
    IncorrectNumberOfFields {
        expected: usize,
        found: usize,
        line_num: usize,
    },
    InvalidStep {
        source: std::num::ParseIntError,
        line_num: usize,
    },
    InvalidNumber {
        num: String,
        line_num: usize,
    },
}

#[derive(Debug, PartialEq)]
pub enum OutputFileWriteError {
    InvalidOutputFileType,
    Io(IoError),
}

#[derive(Debug)]
pub struct IoError(pub std::io::Error);

impl PartialEq for IoError {
    fn eq(&self, other: &Self) -> bool {
        self.0.kind() == other.0.kind()
    }
}

impl std::fmt::Display for InputFileReadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InputFileReadError::InvalidInputFileType => {
                write!(f, "Runtime Error: Invalid input file type.")
            }

            InputFileReadError::Io(io_error) => {
                write!(f, "{io_error}")
            }

            InputFileReadError::InvalidHeader => {
                write!(
                    f,
                    "Runtime Error: Invalid input file header. First line should be: \"step entity value\" (Whitespace variable)"
                )
            }

            InputFileReadError::IncorrectNumberOfFields {
                expected,
                found,
                line_num,
            } => {
                write!(
                    f,
                    "Runtime Error: Incorrect number of fields in input file. Expected: {}, found: {}\nat line {}",
                    expected, found, line_num
                )
            }

            InputFileReadError::InvalidStep { source, line_num } => {
                write!(
                    f,
                    "Runtime Error: Invalid step value in input file at line {}.\nat line {}",
                    source, line_num
                )
            }

            InputFileReadError::InvalidNumber { num, line_num } => {
                write!(
                    f,
                    "Runtime Error: Invalid number {} in putput file.\nat line {}",
                    num, line_num
                )
            }
        }
    }
}

impl std::fmt::Display for OutputFileWriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFileWriteError::InvalidOutputFileType => {
                write!(f, "Runtime Error: Invalid output file type")
            }

            OutputFileWriteError::Io(io_error) => {
                write!(f, "{io_error}")
            }
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "I/O error accessing file: {}", self.0)
    }
}

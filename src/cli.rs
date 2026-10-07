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

//! # cli
//!
//! handles the project command line interface
//!
//! ## Invariants
//!
//! Author: Cole Francis

use clap::{Parser, Subcommand};
use std::process::ExitCode;

use crate::{
    compiler::{Compiler, error_handling::compile_error::CompileError}, 
    simulator::{
        Simulator, 
        io_file::IoFile,
        error_handling::{
            runtime_diagnostics::RuntimeError,
            io_file_error::{InputFileReadError, OutputFileWriteError},
        },
    }
};


////////////////////////////////////////////////////////////////////////////////
// CLI definitions
////////////////////////////////////////////////////////////////////////////////
#[derive(Parser, Debug)]
#[command(
    name = "loi",
    version,
    about = "LOI research tool for emergent computation"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    // /// Copmile a .loi file into a .loic file
    // Compile {
    //     /// Input .loi file
    //     input: String,

    //     /// Top-level net
    //     #[arg(short, long)]
    //     top: String,

    //     /// Output .loic file
    //     #[arg(short, long)]
    //     output: String
    // },

    // /// Start an interactive simulation
    // Simulate {
    //     /// Input .loi or .loic file
    //     input: String,

    //     /// Top-level net, required when compiling a .loi file
    //     #[arg(short, long)]
    //     top: Option<String>,
    // },

    /// Run a complete simulation from an input file and write output
    SimulateFull {
        /// Input .loi or .loic file
        loi_file: String,

        /// Simulation input file
        inputs: String,

        /// Simulation output file
        outputs: String,

        /// Maximum number of simulation steps
        steps: usize,

        /// Top-level net, required when compiling a .loi file
        top: String,
    },
}

////////////////////////////////////////////////////////////////////////////////
// CLI Errors
////////////////////////////////////////////////////////////////////////////////
#[derive(Debug)]
pub enum CliError {
    Compile(CompileError),
    Runtime(RuntimeError),
    Input(InputFileReadError),
    Output(OutputFileWriteError),
}

impl From<CompileError> for CliError {
    fn from(err: CompileError) -> Self {
        Self::Compile(err)
    }
}

impl From<RuntimeError> for CliError {
    fn from(err: RuntimeError) -> Self {
        Self::Runtime(err)
    }
}

impl From<InputFileReadError> for CliError {
    fn from(err: InputFileReadError) -> Self {
        Self::Input(err)
    }
}

impl From<OutputFileWriteError> for CliError {
    fn from(err: OutputFileWriteError) -> Self {
        Self::Output(err)
    }
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compile(err) => write!(f, "{err}"),
            Self::Runtime(err) => write!(f, "{err}"),
            Self::Input(err)   => write!(f, "{err}"),
            Self::Output(err)  => write!(f, "{err}"),
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Command Dispatch
////////////////////////////////////////////////////////////////////////////////
pub fn run () -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.command {
        // Command::Compile {input, top, output} => compile(input, top, output),

        // Command::Simulate {}

        Command::SimulateFull {loi_file, top, inputs, outputs, steps} => 
            simulate_full(&loi_file, &top, &inputs, &outputs, steps),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::FAILURE
        }
    }
}

////////////////////////////////////////////////////////////////////////////////
// Command implementations
////////////////////////////////////////////////////////////////////////////////
fn simulate_full(file_path: &str, top_net: &str, input_file_path: &str, output_file_path: &str, steps: usize) -> Result<(), CliError> {
    let (netlist, interface, relations, inits) = Compiler::compile(file_path, top_net)?;

    let mut sim = Simulator::new(netlist, interface, relations, inits);

    let inputs = IoFile::read(input_file_path)?;

    sim.load_inputs(inputs)?;

    let ran_steps = sim.run(steps, false)?;
    
    let outputs = sim.dump_outputs();

    println!("Outputs: {:?}", outputs);

    IoFile::write(output_file_path, outputs)?;

    Ok(())
}
use crate::{compile_source, compile_source_to_vhdl};
use std::{fmt, fs, path::PathBuf, process::Command};

#[derive(Debug, Clone, Default)]
pub struct SimulationOptions {
    pub testbench: Option<String>,
    pub vcd_path: Option<PathBuf>,
    pub ghdl_path: Option<PathBuf>,
    pub work_directory: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct SimulationResult {
    pub testbench_name: String,
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub generated_vhdl: PathBuf,
    pub vcd_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationStage {
    Detection,
    Selection,
    Generation,
    Analyze,
    Elaborate,
    Run,
}

#[derive(Debug)]
pub struct SimulationError {
    pub stage: SimulationStage,
    pub message: String,
}
impl fmt::Display for SimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for SimulationError {}

pub fn simulate_source(
    source: &str,
    options: &SimulationOptions,
) -> Result<Vec<SimulationResult>, SimulationError> {
    let program = compile_source(source)
        .map_err(|error| simulation_error(SimulationStage::Generation, error.to_string()))?;
    if program.testbenches.is_empty() {
        return Err(simulation_error(
            SimulationStage::Selection,
            "source contains no testbenches",
        ));
    }
    let selected = program
        .testbenches
        .iter()
        .filter(|tb| {
            options
                .testbench
                .as_ref()
                .is_none_or(|name| name == &tb.name)
        })
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(simulation_error(
            SimulationStage::Selection,
            "requested testbench was not found",
        ));
    }
    let vhdl = compile_source_to_vhdl(source)
        .map_err(|error| simulation_error(SimulationStage::Generation, error.to_string()))?;
    let ghdl = options
        .ghdl_path
        .clone()
        .unwrap_or_else(|| PathBuf::from("ghdl"));
    let version = Command::new(&ghdl)
        .arg("--version")
        .output()
        .map_err(|error| {
            simulation_error(
                SimulationStage::Detection,
                format!("GHDL is not available: {error}"),
            )
        })?;
    if !version.status.success() {
        return Err(simulation_error(
            SimulationStage::Detection,
            "GHDL --version failed",
        ));
    }
    let original_dir = std::env::current_dir()
        .map_err(|error| simulation_error(SimulationStage::Generation, error.to_string()))?;
    let mut results = Vec::new();
    for testbench in selected {
        let base = options
            .work_directory
            .clone()
            .unwrap_or_else(|| PathBuf::from("target/gatelisp-sim"));
        let work = base.join(sanitize(&testbench.name));
        fs::create_dir_all(&work).map_err(|error| {
            simulation_error(
                SimulationStage::Generation,
                format!("cannot create {}: {error}", work.display()),
            )
        })?;
        let generated = work.join("generated.vhd");
        fs::write(&generated, &vhdl).map_err(|error| {
            simulation_error(
                SimulationStage::Generation,
                format!("cannot write {}: {error}", generated.display()),
            )
        })?;
        run_ghdl(
            &ghdl,
            &work,
            SimulationStage::Analyze,
            &["-a", "--std=08", "generated.vhd"],
        )?;
        let entity = format!("gl_tb{}_{}", testbench.id.0, sanitize(&testbench.name));
        run_ghdl(
            &ghdl,
            &work,
            SimulationStage::Elaborate,
            &["-e", "--std=08", &entity],
        )?;
        let vcd = options.vcd_path.as_ref().map(|path| {
            if path.is_absolute() {
                path.clone()
            } else {
                original_dir.join(path)
            }
        });
        let mut arguments = vec![
            "-r".to_owned(),
            "--std=08".to_owned(),
            entity,
            "--assert-level=error".to_owned(),
        ];
        if let Some(path) = &vcd {
            arguments.push(format!("--vcd={}", path.display()));
        }
        let refs = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        let output = Command::new(&ghdl)
            .args(&refs)
            .current_dir(&work)
            .output()
            .map_err(|error| simulation_error(SimulationStage::Run, error.to_string()))?;
        if !output.status.success() {
            return Err(simulation_error(
                SimulationStage::Run,
                format!(
                    "GHDL run failed for {}:\n{}",
                    testbench.name,
                    String::from_utf8_lossy(&output.stderr)
                ),
            ));
        }
        results.push(SimulationResult {
            testbench_name: testbench.name.clone(),
            success: true,
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            generated_vhdl: generated,
            vcd_path: vcd,
        });
    }
    Ok(results)
}

fn run_ghdl(
    ghdl: &PathBuf,
    work: &PathBuf,
    stage: SimulationStage,
    arguments: &[&str],
) -> Result<(), SimulationError> {
    let output = Command::new(ghdl)
        .args(arguments)
        .current_dir(work)
        .output()
        .map_err(|error| simulation_error(stage, error.to_string()))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(simulation_error(
            stage,
            format!(
                "GHDL {stage:?} failed:\n{}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ))
    }
}
fn simulation_error(stage: SimulationStage, message: impl Into<String>) -> SimulationError {
    SimulationError {
        stage,
        message: message.into(),
    }
}
fn sanitize(name: &str) -> String {
    let value = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    if value.is_empty() { "x".into() } else { value }
}

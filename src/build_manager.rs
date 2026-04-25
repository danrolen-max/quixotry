// build_manager.rs - Cargo build manager with logging and error handling
//
// Usage: rustc build_manager.rs && ./build_manager [build|run [args...]]
//
// This module provides a build management system that:
// - Executes cargo build with proper error handling
// - Logs all build output with timestamps
// - Captures and reports errors
// - Can optionally run the built binary

use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::SystemTime;

/// Logger handles timestamped logging to both file and stdout
struct Logger {
    log_file: Option<File>,
    verbose: bool,
}

impl Logger {
    /// Create a new logger with optional file output
    fn new(log_path: &str, verbose: bool) -> Result<Self, String> {
        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)
            .map_err(|e| format!("Failed to create log file: {}", e))?;

        Ok(Self {
            log_file: Some(log_file),
            verbose,
        })
    }

    /// Get formatted timestamp
    fn timestamp() -> String {
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => format!("[{}.{:03}]", d.as_secs(), d.subsec_millis()),
            Err(_) => "[unknown]".to_string(),
        }
    }

    /// Log a message with level
    fn log(&mut self, level: &str, message: &str) {
        let timestamp = Self::timestamp();
        let formatted = format!("{} [{}] {}\n", timestamp, level, message);

        // Write to log file
        if let Some(ref mut f) = self.log_file {
            if let Err(e) = f.write_all(formatted.as_bytes()) {
                eprintln!("Failed to write to log: {}", e);
            }
            let _ = f.flush();
        }

        // Print to console based on level
        if self.verbose || level == "ERROR" || level == "WARN" {
            print!("{}", formatted);
            if let Err(e) = std::io::stdout().flush() {
                eprintln!("Failed to flush stdout: {}", e);
            }
        }
    }

    fn info(&mut self, msg: &str) {
        self.log("INFO", msg);
    }
    fn warn(&mut self, msg: &str) {
        self.log("WARN", msg);
    }
    fn error(&mut self, msg: &str) {
        self.log("ERROR", msg);
    }
    fn debug(&mut self, msg: &str) {
        self.log("DEBUG", msg);
    }
}

/// BuildManager orchestrates cargo builds with logging
struct BuildManager {
    project_root: String,
    logger: Logger,
}

impl BuildManager {
    fn new(project_root: &str, log_path: &str, verbose: bool) -> Result<Self, String> {
        Ok(Self {
            project_root: project_root.to_string(),
            logger: Logger::new(log_path, verbose)?,
        })
    }

    /// Execute cargo build
    fn build(&mut self) -> Result<String, String> {
        self.logger
            .info("===========================================");
        self.logger.info("Starting cargo build");
        self.logger
            .info("===========================================");

        let mut cmd = Command::new("cargo");
        cmd.arg("build")
            .current_dir(&self.project_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn cargo: {}", e))?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| "Failed to capture stdout".to_string())?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| "Failed to capture stderr".to_string())?;

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // Read stdout line by line
        for line in stdout_reader.lines() {
            match line {
                Ok(line_content) => {
                    self.logger.debug(&line_content);
                }
                Err(e) => {
                    self.logger.warn(&format!("Error reading stdout: {}", e));
                }
            }
        }

        // Read stderr line by line
        for line in stderr_reader.lines() {
            match line {
                Ok(line_content) => {
                    self.logger.debug(&line_content);
                }
                Err(e) => {
                    self.logger.warn(&format!("Error reading stderr: {}", e));
                }
            }
        }

        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait for cargo: {}", e))?;

        if !status.success() {
            let exit_code = status.code().unwrap_or(-1);
            let err_msg = format!("Build FAILED with exit code: {}", exit_code);
            self.logger.error(&err_msg);
            return Err(err_msg);
        }

        self.logger.info("Build completed SUCCESSFULLY");

        // Find the built binary
        let binary_path = self.find_binary()?;
        self.logger
            .info(&format!("Binary located at: {}", binary_path));

        Ok(binary_path)
    }

    /// Find the built binary path
    fn find_binary(&self) -> Result<String, String> {
        let binary_name = self.get_binary_name();

        let debug_path = format!("{}/target/debug/{}", self.project_root, binary_name);
        if Path::new(&debug_path).exists() {
            return Ok(debug_path);
        }

        let release_path = format!("{}/target/release/{}", self.project_root, binary_name);
        if Path::new(&release_path).exists() {
            return Ok(release_path);
        }

        Err(format!(
            "No binary found: looked for {} in target/debug and target/release",
            binary_name
        ))
    }

    /// Get the binary name from Cargo.toml
    fn get_binary_name(&self) -> String {
        let cargo_path = format!("{}/Cargo.toml", self.project_root);

        if let Ok(content) = std::fs::read_to_string(&cargo_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("name") && trimmed.contains('=') {
                    if let Some(name) = trimmed.split('=').nth(1) {
                        return name.trim().trim_matches('"').trim().to_string();
                    }
                }
            }
        }

        // Default fallback
        "quixotry".to_string()
    }

    /// Run the built binary with arguments
    fn run(&mut self, args: &[String]) -> Result<(), String> {
        let binary_path = self.build()?;

        self.logger
            .info(&format!("Running: {} {:?}", binary_path, args));

        let mut cmd = Command::new(&binary_path);
        cmd.args(args);
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd
            .status()
            .map_err(|e| format!("Failed to run binary: {}", e))?;

        if !status.success() {
            let code = status.code().unwrap_or(-1);
            let err_msg = format!("Binary exited with code: {}", code);
            self.logger.error(&err_msg);
            return Err(err_msg);
        }

        self.logger.info("Binary execution completed");
        Ok(())
    }
}

fn main() {
    // Get arguments
    let args: Vec<String> = env::args().collect();

    // Default to build if no args given
    let (action, _remaining_args) = args.split_at(1);

    let action = if args.len() < 2 { "build" } else { &args[1] };

    // Determine project root (assume we're running from project root or src/)
    let project_root = if Path::new("Cargo.toml").exists() {
        ".".to_string()
    } else if Path::new("../Cargo.toml").exists() {
        "..".to_string()
    } else {
        eprintln!("Error: Cannot find Cargo.toml in current or parent directory");
        std::process::exit(1);
    };

    // Create logger (logs to build.log)
    let mut logger = match Logger::new("build.log", true) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to create logger: {}", e);
            std::process::exit(1);
        }
    };

    logger.info("===========================================");
    logger.info("Quixotry RNG Build Manager");
    logger.info(&format!("Project root: {}", project_root));
    logger.info(&format!("Action: {}", action));
    logger.info("===========================================");

    let mut manager = match BuildManager::new(&project_root, "build.log", true) {
        Ok(m) => m,
        Err(e) => {
            logger.error(&format!("Failed to create build manager: {}", e));
            std::process::exit(1);
        }
    };

    match action {
        "build" => match manager.build() {
            Ok(binary) => {
                logger.info(&format!("BUILD SUCCESS: {}", binary));
                println!("\n✓ Build successful! Binary: {}", binary);
            }
            Err(e) => {
                logger.error(&format!("BUILD FAILED: {}", e));
                eprintln!("\n✗ Build failed. Check build.log for details.");
                std::process::exit(1);
            }
        },
        "run" => {
            let run_args = if args.len() > 2 { &args[2..] } else { &[] };

            match manager.run(run_args) {
                Ok(()) => {
                    logger.info("RUN SUCCESS");
                    println!("\n✓ Execution successful!");
                }
                Err(e) => {
                    logger.error(&format!("RUN FAILED: {}", e));
                    eprintln!("\n✗ Execution failed. Check build.log for details.");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            logger.error(&format!("Unknown action: {}", action));
            eprintln!("Usage: {} [build|run] [args...]", args[0]);
            eprintln!("  build - Build the project");
            eprintln!("  run   - Build and run with optional args");
            std::process::exit(1);
        }
    }
}

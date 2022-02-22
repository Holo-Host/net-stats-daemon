// mod types;

// use types::{ Payload };
use std::process::Command;
use log::{debug, info};

fn main() {
  info!("Collecting payload from holoport");
  println!("Result: '{:?}'", get_value());
}

/// Collect info from holoport by executing bash command
/// Return stdout, in case of a failure in execution or non-zero
/// exit status log error and return Null
fn get_value() -> Option<String> {
  let result = Command::new("echo")
    .arg("abba")
    .arg("|")
    .arg("grep")
    .arg("xx")
    .output();

  let output = match result {
    Ok(x) => {
      if x.status.success() {
        x.stdout
      } else {
        debug!("Failed to get abba, {}", String::from_utf8_lossy(&x.stderr).to_string());
        return None
      }
    },
    Err(_) => {
      debug!("Failed to execute echo");
      return None
    }
  };
  Some(String::from_utf8_lossy(&output).to_string())
}
